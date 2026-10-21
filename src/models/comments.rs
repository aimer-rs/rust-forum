use crate::api::BASE_URL;
use crate::html_markdown::html_to_markdown;
use crate::models::posts::Post;
use std::rc::Rc;

/// One reply in a topic, ready to render: the post's HTML is converted to
/// Markdown once here rather than on every rebuild.
#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    pub post_number: i64,
    /// The `post_number` this comment replies to, if it replies to another
    /// comment rather than to the topic.
    pub reply_to: Option<i64>,
    pub username: String,
    pub avatar_url: String,
    pub created_at: String,
    pub like_count: i64,
    pub markdown: Rc<str>,
}

impl Comment {
    pub fn from_post(post: &Post) -> Self {
        Self {
            post_number: post.post_number,
            reply_to: post.reply_to(),
            username: post.username.clone(),
            avatar_url: avatar_url(&post.avatar_template, AVATAR_SIZE),
            created_at: post.created_at.clone(),
            like_count: post.like_count(),
            markdown: html_to_markdown(&post.cooked),
        }
    }
}

/// The pixel size avatars are requested at (drawn at half this on screen, for
/// sharpness on high-density displays).
const AVATAR_SIZE: u32 = 72;

/// Resolves a Discourse `avatar_template` into a fetchable URL.
///
/// Templates hold a `{size}` placeholder and are either site-relative
/// (`/user_avatar/…`) or already absolute (letter avatars on the CDN).
pub fn avatar_url(template: &str, size: u32) -> String {
    let path = template.replace("{size}", &size.to_string());
    if path.starts_with("http://") || path.starts_with("https://") {
        path
    } else if path.starts_with("//") {
        format!("https:{path}")
    } else {
        format!("{BASE_URL}/{}", path.trim_start_matches('/'))
    }
}

/// A comment together with the comments that reply to it.
#[derive(Debug, PartialEq)]
pub struct CommentNode {
    pub comment: Rc<Comment>,
    pub replies: Vec<Rc<CommentNode>>,
}

impl CommentNode {
    /// How many comments sit below this one, at any depth.
    pub fn descendant_count(&self) -> usize {
        self.replies
            .iter()
            .map(|reply| 1 + reply.descendant_count())
            .sum()
    }
}

/// Groups comments under the comment each one replies to.
///
/// `comments` must be in post order, which is how Discourse streams them, so a
/// parent always precedes its replies. A comment with no parent, a reply to the
/// topic's first post (`OP_POST_NUMBER`), or a reply to a post that isn't
/// loaded becomes a top-level comment.
pub fn build_comment_tree(comments: &[Rc<Comment>]) -> Vec<Rc<CommentNode>> {
    const OP_POST_NUMBER: i64 = 1;

    // `children[i]` lists the indexes of the comments replying to `comments[i]`.
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); comments.len()];
    let mut roots = Vec::new();
    for (index, comment) in comments.iter().enumerate() {
        let parent = comment
            .reply_to
            .filter(|&number| number != OP_POST_NUMBER && number != comment.post_number)
            .and_then(|number| {
                // Only an earlier comment can be a parent; this also keeps a
                // malformed cycle from ever forming.
                comments[..index]
                    .iter()
                    .rposition(|candidate| candidate.post_number == number)
            });
        match parent {
            Some(parent) => children[parent].push(index),
            None => roots.push(index),
        }
    }

    fn assemble(
        index: usize,
        comments: &[Rc<Comment>],
        children: &[Vec<usize>],
    ) -> Rc<CommentNode> {
        Rc::new(CommentNode {
            comment: comments[index].clone(),
            replies: children[index]
                .iter()
                .map(|&child| assemble(child, comments, children))
                .collect(),
        })
    }

    roots
        .into_iter()
        .map(|root| assemble(root, comments, &children))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(post_number: i64, reply_to: Option<i64>) -> Rc<Comment> {
        Rc::new(Comment {
            post_number,
            reply_to,
            username: format!("user{post_number}"),
            avatar_url: String::new(),
            created_at: String::new(),
            like_count: 0,
            markdown: Rc::from(""),
        })
    }

    fn numbers(nodes: &[Rc<CommentNode>]) -> Vec<i64> {
        nodes.iter().map(|node| node.comment.post_number).collect()
    }

    #[test]
    fn replies_nest_under_their_parent() {
        // Mirrors the shape of a real thread: 3 <- 4, 5 <- 6 <- {7, 8 <- 9}.
        let comments = [
            comment(2, None),
            comment(3, None),
            comment(4, Some(3)),
            comment(5, None),
            comment(6, Some(5)),
            comment(7, Some(6)),
            comment(8, Some(6)),
            comment(9, Some(8)),
        ]
        .to_vec();
        let tree = build_comment_tree(&comments);

        assert_eq!(numbers(&tree), [2, 3, 5]);
        assert_eq!(numbers(&tree[1].replies), [4]);
        let five = &tree[2];
        assert_eq!(numbers(&five.replies), [6]);
        assert_eq!(numbers(&five.replies[0].replies), [7, 8]);
        assert_eq!(numbers(&five.replies[0].replies[1].replies), [9]);
        assert_eq!(five.descendant_count(), 4);
        assert_eq!(tree[0].descendant_count(), 0);
    }

    #[test]
    fn replies_to_the_topic_or_to_missing_posts_stay_top_level() {
        let comments = [
            comment(2, Some(1)),
            comment(3, Some(40)),
            comment(4, Some(4)),
            comment(5, Some(9)),
        ]
        .to_vec();
        assert_eq!(numbers(&build_comment_tree(&comments)), [2, 3, 4, 5]);
    }

    #[test]
    fn avatar_templates_resolve_to_absolute_urls() {
        assert_eq!(
            avatar_url("/user_avatar/users.rust-lang.org/zicog/{size}/12216_2.png", 72),
            "https://users.rust-lang.org/user_avatar/users.rust-lang.org/zicog/72/12216_2.png"
        );
        assert_eq!(
            avatar_url("https://avatars.discourse-cdn.com/v4/letter/a/aeb1/{size}.png", 72),
            "https://avatars.discourse-cdn.com/v4/letter/a/aeb1/72.png"
        );
    }
}
