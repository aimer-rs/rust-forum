use serde::Deserialize;

/// Response from `u/{username}/summary.json`: a user's activity at a glance.
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct UserSummaryRoot {
    /// The user's most-liked topics.
    pub topics: Vec<SummaryTopic>,
    /// Every badge the summary mentions, by id.
    pub badges: Vec<Badge>,
    pub user_summary: UserSummary,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct SummaryTopic {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub posts_count: i64,
    pub category_id: i64,
    pub like_count: i64,
    pub created_at: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct Badge {
    pub id: i64,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct UserSummary {
    pub likes_given: i64,
    pub likes_received: i64,
    pub topics_entered: i64,
    pub posts_read_count: i64,
    pub days_visited: i64,
    pub topic_count: i64,
    pub post_count: i64,
    /// Seconds spent reading.
    pub time_read: i64,
    pub solved_count: i64,
    pub top_categories: Vec<TopCategory>,
    pub badges: Vec<GrantedBadge>,
    /// The links the user posts most clicked.
    pub links: Vec<SummaryLink>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct SummaryLink {
    pub url: String,
    pub title: Option<String>,
    pub clicks: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct TopCategory {
    pub id: i64,
    pub name: String,
    /// Hex color without the leading `#`, e.g. `F7941D`.
    pub color: String,
    pub slug: String,
    pub topic_count: i64,
    pub post_count: i64,
}

/// A badge the user holds, and how many times it was granted.
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct GrantedBadge {
    pub badge_id: i64,
    pub count: i64,
}

/// Response from `u/{username}.json`: who the user is.
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct UserRoot {
    pub user: UserDetail,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct UserDetail {
    pub id: i64,
    pub username: String,
    pub name: Option<String>,
    pub avatar_template: String,
    pub created_at: String,
    pub last_seen_at: Option<String>,
    pub title: Option<String>,
    pub trust_level: i64,
    pub admin: bool,
    pub moderator: bool,
    pub bio_cooked: Option<String>,
    pub badge_count: i64,
    pub profile_view_count: i64,
}

/// Which of a user's activity a list shows: the kinds Discourse files under
/// `user_actions.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionFilter {
    /// Topics the user started.
    Topics,
    /// Posts the user wrote in other topics.
    Replies,
    /// Posts the user liked.
    Likes,
}

impl ActionFilter {
    /// The `filter` Discourse takes for it.
    pub fn code(self) -> u8 {
        match self {
            Self::Topics => 4,
            Self::Replies => 5,
            Self::Likes => 1,
        }
    }
}

/// Response from `user_actions.json`: a page of one kind of activity.
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct UserActionsRoot {
    pub user_actions: Vec<UserAction>,
}

/// One entry of a user's activity: a topic they started, a post they wrote or a
/// post they liked.
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct UserAction {
    pub title: String,
    pub slug: String,
    pub topic_id: i64,
    /// A few words of the post, as HTML text with entities.
    pub excerpt: Option<String>,
    pub created_at: String,
    pub category_id: i64,
    /// The author of the post. For a like, that is not the user whose page it is.
    pub username: String,
}

/// Everything the profile screen shows, from the two requests that make it.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct UserProfile {
    pub detail: UserDetail,
    pub summary: UserSummaryRoot,
}

impl UserProfile {
    /// The name of a badge the summary lists, by id.
    pub fn badge_name(&self, id: i64) -> Option<&str> {
        self.summary
            .badges
            .iter()
            .find(|badge| badge.id == id)
            .map(|badge| badge.name.as_str())
    }

    /// The label for the user's standing: staff first, then trust level.
    pub fn role(&self) -> String {
        if self.detail.admin {
            "Admin".to_string()
        } else if self.detail.moderator {
            "Moderator".to_string()
        } else {
            format!("Trust level {}", self.detail.trust_level)
        }
    }
}

/// Seconds of reading as a short duration: `45m`, `3h 20m`, `2d 5h`.
pub fn format_duration(seconds: i64) -> String {
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;
    if days > 0 {
        format!("{days}d {}h", hours % 24)
    } else if hours > 0 {
        format!("{hours}h {}m", minutes % 60)
    } else {
        format!("{minutes}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_filter_has_its_discourse_code() {
        assert_eq!(ActionFilter::Topics.code(), 4);
        assert_eq!(ActionFilter::Replies.code(), 5);
        assert_eq!(ActionFilter::Likes.code(), 1);
    }

    #[test]
    fn an_action_reads_with_null_and_missing_fields() {
        let root: UserActionsRoot = serde_json::from_str(
            r#"{"user_actions":[{"title":"A topic","slug":"a-topic","topic_id":9,"excerpt":null,"created_at":"2024-08-26T00:17:03.391Z","category_id":1,"username":"quinedot","post_number":3}]}"#,
        )
        .unwrap();
        assert_eq!(root.user_actions[0].topic_id, 9);
        assert!(root.user_actions[0].excerpt.is_none());
    }

    #[test]
    fn durations_use_the_two_largest_units() {
        assert_eq!(format_duration(45 * 60), "45m");
        assert_eq!(format_duration(5213), "1h 26m");
        assert_eq!(format_duration(2 * 86_400 + 5 * 3_600), "2d 5h");
    }

    #[test]
    fn a_summary_reads_with_nulls_and_missing_fields() {
        let root: UserSummaryRoot = serde_json::from_str(
            r#"{"topics":[{"id":1,"title":"t","slug":"t","category_id":5}],
                "user_summary":{"days_visited":24,"top_categories":[{"id":5,"name":"help","color":"F7941D"}]}}"#,
        )
        .unwrap();
        assert_eq!(root.topics[0].id, 1);
        assert_eq!(root.user_summary.days_visited, 24);
        assert_eq!(root.user_summary.top_categories[0].name, "help");
    }

    #[test]
    fn a_user_reads_with_null_optional_fields() {
        let root: UserRoot = serde_json::from_str(
            r#"{"user":{"id":1,"username":"domma","name":"Achim","title":null,"bio_cooked":null,"last_seen_at":null}}"#,
        )
        .unwrap();
        assert_eq!(root.user.username, "domma");
        assert!(root.user.title.is_none());
    }
}
