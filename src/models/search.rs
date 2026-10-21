use super::comments::avatar_url;
use super::{Author, Topic};
use serde::Deserialize;

/// How big the avatars of search results are drawn, in logical pixels.
const AVATAR_SIZE: u32 = 96;

/// What a search looks for: topics (the blog) or people.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchMode {
    #[default]
    Blog,
    User,
}

impl SearchMode {
    /// The modes in the order the toggle shows them.
    pub const ALL: [SearchMode; 2] = [SearchMode::Blog, SearchMode::User];

    pub fn label(self) -> &'static str {
        match self {
            SearchMode::Blog => "Blog",
            SearchMode::User => "User",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|mode| *mode == self).unwrap_or(0)
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or_default()
    }
}

/// Response from `search/query?term=..`: the posts that match, the topics they
/// belong to, and the users whose names match.
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct SearchRoot {
    pub posts: Vec<SearchPost>,
    pub topics: Vec<SearchTopic>,
    pub users: Vec<SearchUser>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct SearchPost {
    pub username: String,
    pub avatar_template: String,
    pub created_at: String,
    pub like_count: i64,
    /// The words around the match.
    pub blurb: String,
    pub topic_id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct SearchTopic {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub created_at: String,
    pub posts_count: i64,
    pub category_id: i64,
    pub like_count: i64,
    pub views: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct SearchUser {
    pub username: String,
    pub name: Option<String>,
    pub avatar_template: String,
}

/// Response from `u/search/users?term=..`.
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct UserSearchRoot {
    pub users: Vec<SearchUser>,
}

/// One topic found by a search, with the words that matched and who wrote them.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct BlogHit {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub blurb: String,
    pub username: String,
    pub avatar_url: String,
    pub created_at: String,
    pub posts_count: i64,
    pub category_id: i64,
    pub like_count: i64,
    pub views: i64,
}

impl BlogHit {
    /// The hit as the topic a feed card draws: the matched words stand in for
    /// the excerpt, and whoever wrote them for the author.
    pub fn to_topic(&self) -> Topic {
        Topic {
            id: self.id,
            slug: self.slug.clone(),
            title: self.title.clone(),
            excerpt: Some(self.blurb.clone()),
            created_at: self.created_at.clone(),
            posts_count: self.posts_count,
            category_id: self.category_id,
            like_count: self.like_count,
            views: self.views,
            last_poster_username: self.username.clone(),
            author: Some(Author {
                username: self.username.clone(),
                avatar_url: self.avatar_url.clone(),
            }),
            ..Default::default()
        }
    }
}

/// One person found by a search.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct UserHit {
    pub username: String,
    pub name: String,
    pub avatar_url: String,
}

/// What a search found, ready to draw.
#[derive(Debug, Clone, PartialEq)]
pub enum SearchResults {
    Blog(Vec<BlogHit>),
    User(Vec<UserHit>),
}

impl SearchResults {
    pub fn is_empty(&self) -> bool {
        match self {
            SearchResults::Blog(hits) => hits.is_empty(),
            SearchResults::User(hits) => hits.is_empty(),
        }
    }
}

impl SearchRoot {
    /// The topics found, in the order Discourse ranked them, each with the first
    /// post that matched it. A topic that none of the posts speaks for still
    /// shows, without a blurb.
    pub fn blog_hits(&self) -> Vec<BlogHit> {
        self.topics
            .iter()
            .map(|topic| {
                let post = self.posts.iter().find(|post| post.topic_id == topic.id);
                BlogHit {
                    id: topic.id,
                    slug: topic.slug.clone(),
                    title: topic.title.clone(),
                    blurb: post.map(|post| post.blurb.clone()).unwrap_or_default(),
                    username: post.map(|post| post.username.clone()).unwrap_or_default(),
                    avatar_url: post
                        .map(|post| avatar_url(&post.avatar_template, AVATAR_SIZE))
                        .unwrap_or_default(),
                    created_at: post
                        .map(|post| post.created_at.clone())
                        .unwrap_or_else(|| topic.created_at.clone()),
                    posts_count: topic.posts_count,
                    category_id: topic.category_id,
                    like_count: topic.like_count,
                    views: topic.views,
                }
            })
            .collect()
    }
}

impl UserSearchRoot {
    pub fn user_hits(&self) -> Vec<UserHit> {
        self.users
            .iter()
            .map(|user| UserHit {
                username: user.username.clone(),
                name: user.name.clone().unwrap_or_default(),
                avatar_url: avatar_url(&user.avatar_template, AVATAR_SIZE),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESPONSE: &str = r#"{
        "posts": [
            {"username":"ferris","avatar_template":"/user_avatar/x/ferris/{size}/1.png",
             "created_at":"2024-01-29T04:56:05.601Z","like_count":2,"blurb":"an apple a day","topic_id":7,"post_number":1}
        ],
        "topics": [
            {"id":7,"title":"Apples","slug":"apples","posts_count":3,"created_at":"2024-01-29T04:56:05.548Z"},
            {"id":8,"title":"Pears","slug":"pears","posts_count":1,"created_at":"2024-02-01T00:00:00.000Z"}
        ],
        "users": [], "categories": [], "grouped_search_result": {"term":"apple"}
    }"#;

    #[test]
    fn a_topic_takes_the_blurb_and_author_of_the_post_that_matched_it() {
        let hits = serde_json::from_str::<SearchRoot>(RESPONSE).unwrap().blog_hits();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].title, "Apples");
        assert_eq!(hits[0].blurb, "an apple a day");
        assert_eq!(hits[0].username, "ferris");
        assert!(hits[0].avatar_url.ends_with("/user_avatar/x/ferris/96/1.png"));
    }

    #[test]
    fn a_topic_without_a_matching_post_still_shows_with_its_own_date() {
        let hits = serde_json::from_str::<SearchRoot>(RESPONSE).unwrap().blog_hits();
        assert_eq!(hits[1].blurb, "");
        assert_eq!(hits[1].username, "");
        assert_eq!(hits[1].created_at, "2024-02-01T00:00:00.000Z");
    }

    #[test]
    fn an_empty_answer_has_no_hits() {
        let root = serde_json::from_str::<SearchRoot>("{}").unwrap();
        assert!(root.blog_hits().is_empty());
    }

    #[test]
    fn users_come_with_their_names_and_avatars() {
        let root = serde_json::from_str::<UserSearchRoot>(
            r#"{"users":[{"id":1,"username":"domma","name":"Achim","avatar_template":"/a/{size}/1.png"},
                         {"id":2,"username":"nameless","name":null,"avatar_template":"/b/{size}/2.png"}]}"#,
        )
        .unwrap();
        let hits = root.user_hits();
        assert_eq!((hits[0].username.as_str(), hits[0].name.as_str()), ("domma", "Achim"));
        assert_eq!(hits[1].name, "");
        assert!(hits[0].avatar_url.ends_with("/a/96/1.png"));
    }

    #[test]
    fn the_toggle_round_trips_through_its_index() {
        for mode in SearchMode::ALL {
            assert_eq!(SearchMode::from_index(mode.index()), mode);
        }
        assert_eq!(SearchMode::from_index(9), SearchMode::Blog);
    }
}
