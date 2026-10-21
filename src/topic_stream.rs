use crate::api::{BASE_URL, session};
use crate::models::comments::avatar_url;
use crate::models::{Author, Topic, User};

/// The pixel size topic-list avatars are requested at (drawn at half this on
/// screen, for sharpness on high-density displays).
const AUTHOR_AVATAR_SIZE: u32 = 48;

/// Incrementally scans a Discourse JSON response for the `topic_list.topics`
/// array and yields each complete topic object as a raw JSON string. This lets
/// topics be deserialized and rendered one-by-one as bytes arrive, rather than
/// waiting for the whole document.
pub struct TopicStreamParser {
    found_array: bool,
    /// The response's `users`, which Discourse sends ahead of the topics.
    users: Vec<User>,
}

impl TopicStreamParser {
    pub fn new() -> Self {
        Self {
            found_array: false,
            users: Vec::new(),
        }
    }

    /// The user who started `topic`: its original poster, else its first one.
    pub fn author_of(&self, topic: &Topic) -> Option<Author> {
        let poster = topic
            .posters
            .iter()
            .find(|poster| poster.description.contains("Original Poster"))
            .or_else(|| topic.posters.first())?;
        let user = self.users.iter().find(|user| user.id == poster.user_id)?;
        Some(Author {
            username: user.username.clone(),
            avatar_url: avatar_url(&user.avatar_template, AUTHOR_AVATAR_SIZE),
        })
    }

    /// Feeds the accumulated `buffer` and returns every topic object that is
    /// now complete, consuming those bytes from `buffer`. Incomplete objects
    /// stay in `buffer` for the next call.
    pub fn poll(&mut self, buffer: &mut Vec<u8>) -> Vec<String> {
        let mut out = Vec::new();
        loop {
            if !self.found_array {
                // Match the key `"topics"` (with its trailing colon) so a value
                // that happens to equal `"topics"` elsewhere is not mistaken for
                // it.
                let Some(key) = find_subslice(buffer, b"\"topics\":") else {
                    break;
                };
                let Some(rel) = buffer[key..].iter().position(|&b| b == b'[') else {
                    break;
                };
                self.users = parse_users(&buffer[..key]);
                buffer.drain(..=key + rel);
                self.found_array = true;
                continue;
            }

            let Some(start) = buffer.iter().position(|&b| b == b'{') else {
                break;
            };
            let Some(end) = find_object_end(&buffer[start..]) else {
                break;
            };
            let end = start + end;
            let json = String::from_utf8(buffer[start..=end].to_vec()).unwrap_or_default();
            out.push(json);
            buffer.drain(..=end);
        }
        out
    }
}

/// Streams the topics of a Discourse path such as `latest.json` or
/// `c/{slug}/{id}.json`, invoking `on_topic` for each topic as soon as its
/// object is fully received.
pub async fn stream_topics(path: &str, on_topic: impl FnMut(Topic)) -> Result<(), String> {
    stream_topics_from(BASE_URL, path, on_topic).await
}

/// [`stream_topics`] against the forum at `base_url`.
async fn stream_topics_from(
    base_url: &str,
    path: &str,
    mut on_topic: impl FnMut(Topic),
) -> Result<(), String> {
    let url = format!("{base_url}/{path}");
    let mut response = session::client()
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        // Discourse serves a 403 (with `error_type: "not_logged_in"`) for lists
        // that require a logged-in user, such as `new.json`. Surface that as a
        // friendly message instead of a bare HTTP status.
        return Err(if response.status() == reqwest::StatusCode::FORBIDDEN {
            "Required to Login".to_string()
        } else {
            format!("HTTP {}", response.status())
        });
    }

    let mut buffer: Vec<u8> = Vec::new();
    let mut parser = TopicStreamParser::new();

    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        buffer.extend_from_slice(&chunk);
        for json in parser.poll(&mut buffer) {
            if let Ok(mut topic) = serde_json::from_str::<Topic>(&json) {
                topic.author = parser.author_of(&topic);
                on_topic(topic);
            }
        }
    }

    Ok(())
}

/// Reads the `users` array out of the text that precedes `topic_list`.
fn parse_users(prefix: &[u8]) -> Vec<User> {
    let Some(key) = find_subslice(prefix, b"\"users\":") else {
        return Vec::new();
    };
    let Some(rel) = prefix[key..].iter().position(|&b| b == b'[') else {
        return Vec::new();
    };
    let start = key + rel;
    let Some(end) = find_object_end(&prefix[start..]) else {
        return Vec::new();
    };
    let Ok(values) = serde_json::from_slice::<Vec<serde_json::Value>>(&prefix[start..=start + end])
    else {
        return Vec::new();
    };
    values
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .collect()
}

/// Finds `needle` in `haystack`, returning its start index.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Given a slice whose first byte is `{`, returns the index of the matching
/// closing `}`, respecting nested objects/arrays and string escapes.
fn find_object_end(bytes: &[u8]) -> Option<usize> {
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;

    for (i, &b) in bytes.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }

        match b {
            b'"' => in_string = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Poster;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn topics_object() -> Vec<String> {
        let mut parser = TopicStreamParser::new();
        let mut buffer = b"{\"users\":[],\"topic_list\":{\"topics\":[{\"id\":1,\"title\":\"a\",\"slug\":\"a\"},{\"id\":2,\"title\":\"b\",\"slug\":\"b\"}]}}".to_vec();
        let mut out = parser.poll(&mut buffer);
        // trailing `]}}` is ignored
        out.extend(parser.poll(&mut buffer));
        out
    }

    #[test]
    fn extracts_each_topic_object() {
        let topics = topics_object();
        assert_eq!(topics.len(), 2);
        assert!(topics[0].starts_with("{\"id\":1"));
        assert!(topics[1].starts_with("{\"id\":2"));
    }

    #[test]
    fn handles_topics_split_across_chunks() {
        let mut parser = TopicStreamParser::new();
        let mut buffer = Vec::new();

        buffer.extend_from_slice(b"{\"users\":[],\"topic_list\":{\"top");
        assert!(parser.poll(&mut buffer).is_empty());

        // The first topic object is still incomplete: no closing brace yet.
        buffer.extend_from_slice(b"ics\":[{\"id\":1,\"title\":\"a\",\"slug\":");
        assert!(parser.poll(&mut buffer).is_empty());

        buffer.extend_from_slice(b"\"a\"},{\"id\":2,\"title\":\"b\",\"slug\":\"b\"}]}}");
        let topics = parser.poll(&mut buffer);
        assert_eq!(topics.len(), 2);
    }

    #[test]
    fn ignores_a_topics_value_before_the_key() {
        let mut parser = TopicStreamParser::new();
        // A user whose username is literally "topics" must not be mistaken for
        // the `"topics"` key.
        let mut buffer =
            br#"{"users":[{"username":"topics"}],"topic_list":{"topics":[{"id":1,"title":"a","slug":"a"}]}}"#
                .to_vec();
        let topics = parser.poll(&mut buffer);
        assert_eq!(topics.len(), 1);
        assert!(topics[0].starts_with("{\"id\":1"));
    }

    #[test]
    fn resolves_a_topics_author_from_the_users_ahead_of_it() {
        let mut parser = TopicStreamParser::new();
        let mut buffer = br#"{"users":[{"id":7,"username":"ferris","avatar_template":"/user_avatar/x/ferris/{size}/1.png","trust_level":1}],"topic_list":{"topics":[{"id":1}]}}"#.to_vec();
        assert_eq!(parser.poll(&mut buffer).len(), 1);

        let poster = |description: &str, user_id| Poster {
            description: description.to_string(),
            user_id,
            ..Default::default()
        };
        let topic = Topic {
            posters: vec![
                poster("Frequent Poster", 9),
                poster("Original Poster, Most Recent Poster", 7),
            ],
            ..Default::default()
        };
        let author = parser.author_of(&topic).unwrap();
        assert_eq!(author.username, "ferris");
        assert!(author.avatar_url.ends_with("/user_avatar/x/ferris/48/1.png"));
    }

    #[test]
    fn ignores_braces_inside_strings() {
        let mut parser = TopicStreamParser::new();
        let mut buffer =
            b"{\"topic_list\":{\"topics\":[{\"id\":1,\"title\":\"a }\",\"slug\":\"a\"}]}}".to_vec();
        let topics = parser.poll(&mut buffer);
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0], "{\"id\":1,\"title\":\"a }\",\"slug\":\"a\"}");
    }

    #[test]
    fn find_object_end_matches_nested_arrays() {
        let json = br#"{"id":1,"posters":[{"x":1}],"nested":{"a":{}}}"#;
        assert_eq!(find_object_end(json), Some(json.len() - 1));
    }

    /// A forum on a local port that lists a topic for a request carrying the
    /// session cookie, and answers anything else the way Discourse answers a
    /// guest asking for `new.json`.
    async fn forum_that_lists_for_members() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let topic = Topic {
            id: 7,
            title: "Members only".to_owned(),
            slug: "members-only".to_owned(),
            ..Default::default()
        };
        let listing = format!(
            "{{\"users\":[],\"topic_list\":{{\"topics\":[{}]}}}}",
            serde_json::to_string(&topic).unwrap()
        );
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else { return };
                let listing = listing.clone();
                tokio::spawn(async move {
                    let mut request = vec![0u8; 8192];
                    let read = socket.read(&mut request).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&request[..read]).to_lowercase();
                    let (status, body) = if request.contains("_t=member-session") {
                        ("200 OK", listing)
                    } else {
                        ("403 Forbidden", "{\"error_type\":\"not_logged_in\"}".to_owned())
                    };
                    let reply = format!(
                        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = socket.write_all(reply.as_bytes()).await;
                });
            }
        });
        format!("http://{address}")
    }

    #[tokio::test]
    async fn a_member_sees_the_list_their_session_cookie_unlocks() {
        let forum = forum_that_lists_for_members().await;
        // What signing in leaves in the shared cookie jar.
        session::jar().add_cookie_str(
            "_t=member-session; Path=/",
            &reqwest::Url::parse(&forum).unwrap(),
        );

        let mut topics = Vec::new();
        let result = stream_topics_from(&forum, "new.json", |topic| topics.push(topic)).await;

        assert_eq!(result, Ok(()));
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].title, "Members only");
    }
}
