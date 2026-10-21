use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// Deserializes a `null` value as the type's `Default` (`0`, `false`, or an
/// empty `String`/`Vec`). Discourse returns `null` for many optional fields, so
/// this lets a bare primitive/owned field accept `null` without wrapping it in
/// `Option`.
fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct PostRoot {
    #[serde(rename = "post_stream")]
    pub post_stream: PostStream,
    #[serde(rename = "timeline_lookup")]
    pub timeline_lookup: Vec<Vec<i64>>,
    #[serde(rename = "suggested_topics")]
    pub suggested_topics: Vec<SuggestedTopic>,
    #[serde(rename = "fancy_title")]
    pub fancy_title: String,
    pub id: i64,
    pub title: String,
    #[serde(rename = "posts_count")]
    pub posts_count: i64,
    #[serde(rename = "created_at")]
    pub created_at: String,
    pub views: i64,
    #[serde(rename = "reply_count")]
    pub reply_count: i64,
    #[serde(rename = "like_count")]
    pub like_count: i64,
    #[serde(rename = "last_posted_at")]
    pub last_posted_at: String,
    pub visible: bool,
    pub closed: bool,
    pub archived: bool,
    #[serde(rename = "has_summary")]
    pub has_summary: bool,
    pub archetype: String,
    pub slug: String,
    #[serde(rename = "category_id")]
    pub category_id: i64,
    #[serde(rename = "word_count")]
    pub word_count: i64,
    #[serde(rename = "deleted_at")]
    pub deleted_at: Value,
    #[serde(rename = "user_id")]
    pub user_id: i64,
    #[serde(rename = "featured_link")]
    pub featured_link: Value,
    #[serde(rename = "pinned_globally")]
    pub pinned_globally: bool,
    #[serde(default, deserialize_with = "null_to_default")]
    #[serde(rename = "pinned_at")]
    pub pinned_at: String,
    #[serde(default, deserialize_with = "null_to_default")]
    #[serde(rename = "pinned_until")]
    pub pinned_until: String,
    #[serde(rename = "image_url")]
    pub image_url: Value,
    #[serde(rename = "slow_mode_seconds")]
    pub slow_mode_seconds: i64,
    pub draft: Value,
    #[serde(rename = "draft_key")]
    pub draft_key: String,
    #[serde(default, deserialize_with = "null_to_default")]
    #[serde(rename = "draft_sequence")]
    pub draft_sequence: i64,
    pub posted: bool,
    pub unpinned: Value,
    pub pinned: bool,
    #[serde(rename = "current_post_number")]
    pub current_post_number: i64,
    #[serde(rename = "highest_post_number")]
    pub highest_post_number: i64,
    #[serde(rename = "last_read_post_number")]
    pub last_read_post_number: i64,
    #[serde(rename = "last_read_post_id")]
    pub last_read_post_id: i64,
    #[serde(rename = "deleted_by")]
    pub deleted_by: Value,
    #[serde(rename = "actions_summary")]
    pub actions_summary: Vec<ActionsSummary2>,
    #[serde(rename = "chunk_size")]
    pub chunk_size: i64,
    pub bookmarked: bool,
    #[serde(rename = "topic_timer")]
    pub topic_timer: Value,
    #[serde(rename = "message_bus_last_id")]
    pub message_bus_last_id: i64,
    #[serde(rename = "participant_count")]
    pub participant_count: i64,
    #[serde(rename = "show_read_indicator")]
    pub show_read_indicator: bool,
    pub thumbnails: Value,
    #[serde(rename = "slow_mode_enabled_until")]
    pub slow_mode_enabled_until: Value,
    #[serde(default, deserialize_with = "null_to_default")]
    #[serde(rename = "related_topics")]
    pub related_topics: Vec<RelatedTopic>,
    pub summarizable: bool,
    #[serde(rename = "has_cached_summary")]
    pub has_cached_summary: bool,
    #[serde(rename = "can_assign", default = "bool::default")]
    pub can_assign: bool,
    #[serde(rename = "has_livestream")]
    pub has_livestream: Value,
    #[serde(rename = "has_accepted_answer")]
    pub has_accepted_answer: bool,
    #[serde(rename = "can_create_shared_issue")]
    pub can_create_shared_issue: bool,
    #[serde(rename = "shared_issue_visible")]
    pub shared_issue_visible: bool,
    #[serde(rename = "can_vote")]
    pub can_vote: bool,
    #[serde(rename = "vote_count")]
    pub vote_count: i64,
    #[serde(rename = "user_voted")]
    pub user_voted: bool,
    #[serde(rename = "discourse_zendesk_plugin_zendesk_id")]
    pub discourse_zendesk_plugin_zendesk_id: Value,
    #[serde(rename = "discourse_zendesk_plugin_zendesk_url")]
    pub discourse_zendesk_plugin_zendesk_url: String,
    pub details: Details,
    #[serde(rename = "pending_posts")]
    pub pending_posts: Vec<Value>,
    pub bookmarks: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct PostStream {
    pub posts: Vec<Post>,
    pub stream: Vec<i64>,
}

/// The response of `t/{id}/posts.json`, which returns a chosen set of a
/// topic's posts. The topic document itself only carries the first batch.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PostsPage {
    #[serde(rename = "post_stream")]
    pub post_stream: PostStream,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub id: i64,
    #[serde(default, deserialize_with = "null_to_default")]
    pub name: String,
    pub username: String,
    #[serde(rename = "avatar_template")]
    pub avatar_template: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
    pub cooked: String,
    #[serde(rename = "post_number")]
    pub post_number: i64,
    #[serde(rename = "post_type")]
    pub post_type: i64,
    #[serde(rename = "posts_count")]
    pub posts_count: i64,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
    #[serde(rename = "reply_count")]
    pub reply_count: i64,
    #[serde(rename = "reply_to_post_number")]
    pub reply_to_post_number: Value,
    #[serde(rename = "quote_count")]
    pub quote_count: i64,
    #[serde(rename = "incoming_link_count")]
    pub incoming_link_count: i64,
    pub reads: i64,
    #[serde(rename = "readers_count")]
    pub readers_count: i64,
    pub score: f64,
    pub yours: bool,
    #[serde(rename = "topic_id")]
    pub topic_id: i64,
    #[serde(rename = "topic_slug")]
    pub topic_slug: String,
    #[serde(default, deserialize_with = "null_to_default")]
    #[serde(rename = "display_username")]
    pub display_username: String,
    #[serde(rename = "primary_group_name")]
    pub primary_group_name: Value,
    #[serde(rename = "flair_name")]
    pub flair_name: Value,
    #[serde(rename = "flair_url")]
    pub flair_url: Value,
    #[serde(rename = "flair_bg_color")]
    pub flair_bg_color: Value,
    #[serde(rename = "flair_color")]
    pub flair_color: Value,
    #[serde(rename = "flair_group_id")]
    pub flair_group_id: Value,
    #[serde(rename = "badges_granted")]
    pub badges_granted: Vec<Value>,
    pub version: i64,
    #[serde(rename = "can_edit")]
    pub can_edit: bool,
    #[serde(rename = "can_delete")]
    pub can_delete: bool,
    #[serde(rename = "can_recover")]
    pub can_recover: bool,
    #[serde(rename = "can_see_hidden_post")]
    pub can_see_hidden_post: bool,
    #[serde(rename = "can_wiki")]
    pub can_wiki: bool,
    #[serde(rename = "link_counts")]
    #[serde(default)]
    pub link_counts: Vec<LinkCount>,
    pub read: bool,
    #[serde(rename = "user_title")]
    pub user_title: Option<String>,
    pub bookmarked: bool,
    #[serde(rename = "actions_summary")]
    pub actions_summary: Vec<ActionsSummary>,
    pub moderator: bool,
    pub admin: bool,
    pub staff: bool,
    #[serde(rename = "user_id")]
    pub user_id: i64,
    pub hidden: bool,
    #[serde(rename = "trust_level")]
    pub trust_level: i64,
    #[serde(rename = "deleted_at")]
    pub deleted_at: Value,
    #[serde(rename = "user_deleted")]
    pub user_deleted: bool,
    #[serde(rename = "edit_reason")]
    pub edit_reason: Value,
    #[serde(rename = "can_view_edit_history")]
    pub can_view_edit_history: bool,
    pub wiki: bool,
    #[serde(rename = "post_url")]
    pub post_url: String,
    #[serde(rename = "can_assign", default = "bool::default")]
    pub can_assign: bool,
    #[serde(rename = "calendar_details")]
    #[serde(default)]
    pub calendar_details: Vec<Value>,
    #[serde(rename = "can_accept_answer")]
    pub can_accept_answer: bool,
    #[serde(rename = "can_unaccept_answer")]
    pub can_unaccept_answer: bool,
    #[serde(rename = "accepted_answer")]
    pub accepted_answer: bool,
    #[serde(rename = "topic_accepted_answer")]
    pub topic_accepted_answer: Value,
    #[serde(rename = "can_vote")]
    pub can_vote: Option<bool>,
    #[serde(rename = "title_is_group")]
    pub title_is_group: Option<bool>,
    #[serde(rename = "user_cakedate")]
    pub user_cakedate: Option<String>,
    #[serde(rename = "user_birthdate")]
    pub user_birthdate: Option<String>,
    #[serde(rename = "action_code")]
    pub action_code: Option<String>,
}

impl Post {
    /// The `post_number` this post replies to, or `None` for a post that
    /// replies to the topic itself (Discourse sends `null`).
    pub fn reply_to(&self) -> Option<i64> {
        self.reply_to_post_number.as_i64()
    }

    /// How many likes the post has: the count of Discourse's "like" action.
    pub fn like_count(&self) -> i64 {
        const LIKE_ACTION_ID: i64 = 2;
        self.actions_summary
            .iter()
            .find(|action| action.id == LIKE_ACTION_ID)
            .and_then(|action| action.count)
            .unwrap_or(0)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LinkCount {
    pub url: String,
    pub internal: bool,
    pub reflection: bool,
    pub title: String,
    pub clicks: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ActionsSummary {
    pub id: i64,
    pub count: Option<i64>,
    #[serde(rename = "can_act", default)]
    pub can_act: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedTopic {
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
    pub pinned: bool,
    pub unpinned: Value,
    pub visible: bool,
    pub closed: bool,
    pub archived: bool,
    pub bookmarked: Value,
    pub liked: Value,
    #[serde(rename = "like_count")]
    pub like_count: i64,
    pub views: i64,
    #[serde(rename = "category_id")]
    pub category_id: i64,
    #[serde(rename = "featured_link")]
    pub featured_link: Value,
    #[serde(rename = "op_like_count")]
    pub op_like_count: i64,
    #[serde(rename = "has_accepted_answer")]
    pub has_accepted_answer: bool,
    pub posters: Vec<Poster>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Poster {
    pub extras: Option<String>,
    pub description: String,
    pub user: User,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(default, deserialize_with = "null_to_default")]
    pub name: String,
    #[serde(rename = "avatar_template")]
    pub avatar_template: String,
    #[serde(rename = "trust_level")]
    pub trust_level: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ActionsSummary2 {
    pub id: i64,
    pub count: i64,
    pub hidden: bool,
    #[serde(rename = "can_act")]
    pub can_act: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct RelatedTopic {
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
    pub pinned: bool,
    pub unpinned: Value,
    pub visible: bool,
    pub closed: bool,
    pub archived: bool,
    pub bookmarked: Value,
    pub liked: Value,
    #[serde(rename = "like_count")]
    pub like_count: i64,
    pub views: i64,
    #[serde(rename = "category_id")]
    pub category_id: i64,
    #[serde(rename = "featured_link")]
    pub featured_link: Value,
    #[serde(rename = "op_like_count")]
    pub op_like_count: i64,
    #[serde(rename = "has_accepted_answer")]
    pub has_accepted_answer: bool,
    pub posters: Vec<Poster2>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Poster2 {
    pub extras: Option<String>,
    pub description: String,
    pub user: User2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct User2 {
    pub id: i64,
    pub username: String,
    #[serde(default, deserialize_with = "null_to_default")]
    pub name: String,
    #[serde(rename = "avatar_template")]
    pub avatar_template: String,
    #[serde(rename = "trust_level")]
    pub trust_level: i64,
    pub admin: Option<bool>,
    pub moderator: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    #[serde(rename = "can_edit")]
    pub can_edit: bool,
    #[serde(rename = "notification_level")]
    pub notification_level: i64,
    #[serde(rename = "notifications_reason_id")]
    pub notifications_reason_id: Value,
    #[serde(rename = "can_invite_to")]
    pub can_invite_to: bool,
    #[serde(rename = "can_reply_as_new_topic")]
    pub can_reply_as_new_topic: bool,
    #[serde(rename = "can_flag_topic")]
    pub can_flag_topic: bool,
    pub participants: Vec<Participant>,
    #[serde(rename = "created_by")]
    pub created_by: CreatedBy,
    #[serde(rename = "last_poster")]
    pub last_poster: LastPoster,
    pub links: Vec<Link>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub id: i64,
    pub username: String,
    #[serde(default, deserialize_with = "null_to_default")]
    pub name: String,
    #[serde(rename = "avatar_template")]
    pub avatar_template: String,
    #[serde(rename = "post_count")]
    pub post_count: i64,
    #[serde(rename = "primary_group_name")]
    pub primary_group_name: Value,
    #[serde(rename = "flair_name")]
    pub flair_name: Value,
    #[serde(rename = "flair_url")]
    pub flair_url: Value,
    #[serde(rename = "flair_color")]
    pub flair_color: Value,
    #[serde(rename = "flair_bg_color")]
    pub flair_bg_color: Value,
    #[serde(rename = "flair_group_id")]
    pub flair_group_id: Value,
    #[serde(rename = "trust_level")]
    pub trust_level: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct CreatedBy {
    pub id: i64,
    pub username: String,
    #[serde(default, deserialize_with = "null_to_default")]
    pub name: String,
    #[serde(rename = "avatar_template")]
    pub avatar_template: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LastPoster {
    pub id: i64,
    pub username: String,
    #[serde(default, deserialize_with = "null_to_default")]
    pub name: String,
    #[serde(rename = "avatar_template")]
    pub avatar_template: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub url: String,
    #[serde(default, deserialize_with = "null_to_default")]
    pub title: String,
    pub internal: bool,
    pub attachment: bool,
    pub reflection: bool,
    pub clicks: i64,
    #[serde(rename = "user_id")]
    pub user_id: i64,
    pub domain: String,
    #[serde(rename = "root_domain")]
    pub root_domain: String,
}

#[cfg(test)]
mod tests {
    use super::PostRoot;

    #[test]
    fn accepts_null_link_titles() {
        let post: PostRoot = serde_json::from_value(serde_json::json!({
            "details": {
                "links": [{
                    "url": "https://dcd.gg/4dplus",
                    "title": null
                }]
            }
        }))
        .expect("null link titles should deserialize");

        assert_eq!(post.details.links[0].title, "");
    }

    #[test]
    fn posts_page_reads_reply_targets() {
        let page: super::PostsPage = serde_json::from_value(serde_json::json!({
            "post_stream": { "posts": [
                { "id": 1, "post_number": 7, "reply_to_post_number": null, "post_type": 1 },
                { "id": 2, "post_number": 8, "reply_to_post_number": 7, "post_type": 1 }
            ] }
        }))
        .expect("a posts page should deserialize");

        let replies: Vec<_> = page.post_stream.posts.iter().map(|post| post.reply_to()).collect();
        assert_eq!(replies, [None, Some(7)]);
    }
}
