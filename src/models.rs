pub mod categories;
pub mod comments;
pub mod posts;
pub mod search;
pub mod user;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::rc::Rc;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub users: Vec<User>,
    #[serde(rename = "primary_groups")]
    pub primary_groups: Vec<Value>,
    #[serde(rename = "flair_groups")]
    pub flair_groups: Vec<Value>,
    #[serde(rename = "topic_list")]
    pub topic_list: Rc<TopicList>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i64,
    pub username: String,
    pub name: Option<String>,
    #[serde(rename = "avatar_template")]
    pub avatar_template: String,
    #[serde(rename = "trust_level")]
    pub trust_level: i64,
    pub admin: Option<bool>,
    pub moderator: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicList {
    #[serde(rename = "can_create_topic")]
    pub can_create_topic: bool,
    #[serde(rename = "more_topics_url")]
    pub more_topics_url: String,
    #[serde(rename = "per_page")]
    pub per_page: i64,
    pub topics: Vec<Topic>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Topic {
    #[serde(rename = "fancy_title")]
    pub fancy_title: String,
    pub id: i64,
    pub title: String,
    pub slug: String,
    #[serde(rename = "posts_count")]
    pub posts_count: i64,
    #[serde(rename = "reply_count")]
    pub reply_count: i64,
    #[serde(rename = "highest_post_number")]
    pub highest_post_number: i64,
    #[serde(rename = "image_url")]
    pub image_url: Option<String>,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "last_posted_at")]
    pub last_posted_at: String,
    pub bumped: bool,
    #[serde(rename = "bumped_at")]
    pub bumped_at: String,
    pub archetype: String,
    pub unseen: bool,
    #[serde(rename = "last_read_post_number")]
    pub last_read_post_number: Option<i64>,
    pub unread: Option<i64>,
    #[serde(rename = "new_posts")]
    pub new_posts: Option<i64>,
    #[serde(rename = "unread_posts")]
    pub unread_posts: Option<i64>,
    pub pinned: bool,
    pub unpinned: Value,
    pub excerpt: Option<String>,
    pub visible: bool,
    pub closed: bool,
    pub archived: bool,
    #[serde(rename = "notification_level")]
    pub notification_level: Option<i64>,
    pub bookmarked: Option<bool>,
    pub liked: Option<bool>,
    pub views: i64,
    #[serde(rename = "like_count")]
    pub like_count: i64,
    #[serde(rename = "has_summary")]
    pub has_summary: bool,
    #[serde(rename = "last_poster_username")]
    pub last_poster_username: String,
    #[serde(rename = "category_id")]
    pub category_id: i64,
    #[serde(rename = "op_like_count")]
    pub op_like_count: i64,
    #[serde(rename = "pinned_globally")]
    pub pinned_globally: bool,
    #[serde(rename = "featured_link")]
    pub featured_link: Value,
    #[serde(rename = "can_assign", default = "bool::default")]
    pub can_assign: bool,
    #[serde(rename = "has_accepted_answer")]
    pub has_accepted_answer: bool,
    #[serde(rename = "can_vote")]
    pub can_vote: bool,
    pub posters: Vec<Poster>,
    #[serde(rename = "unicode_title")]
    pub unicode_title: Option<String>,
    #[serde(rename = "visibility_reason_id")]
    pub visibility_reason_id: Option<i64>,
    /// Who started the topic, resolved from the list's `users` while streaming;
    /// not part of Discourse's topic JSON.
    #[serde(skip)]
    pub author: Option<Author>,
}

/// The user who started a topic: a name and an avatar to draw beside it.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Author {
    pub username: String,
    pub avatar_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Poster {
    pub extras: Option<String>,
    pub description: String,
    #[serde(rename = "user_id")]
    pub user_id: i64,
    #[serde(rename = "primary_group_id")]
    pub primary_group_id: Value,
    #[serde(rename = "flair_group_id")]
    pub flair_group_id: Value,
}
