use crate::api::Api;
use crate::models::categories::{Category, CategoryRoot};
use crate::models::comments::Comment;
use crate::models::posts::PostRoot;
use crate::models::search::{SearchMode, SearchResults};
use crate::preferences::NavbarTabs;
use crate::models::user::{ActionFilter, UserAction, UserProfile};
use crate::models::{Root, Topic};
use aimer::share::{RcProjectExt, ShareRef, Shared};
use aimer::style::ThemeMode;
use std::collections::HashMap;
use std::rc::Rc;

/// What the dialog window is first showing when it opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogPage {
    Settings,
    Login,
}

/// Which of the app's own windows are open, on a desktop. They are windows of the
/// operating system rather than parts of the app's window; see
/// [`crate::components::windows`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Windows {
    /// The one window the settings and the sign-in share, and the page it opened
    /// on; `None` while it is shut.
    pub dialog: Option<DialogPage>,
    /// The window with the signed-in member's own profile.
    pub profile: bool,
}

#[derive(Clone, Default)]
pub struct AppStore {
    pub root: Root,
    pub posts: HashMap<i64, PostRoot>,
    /// Loaded user profiles, by username.
    pub profiles: HashMap<String, UserProfile>,
    /// A user's loaded activity, by username and kind.
    pub actions: HashMap<(String, ActionFilter), Vec<UserAction>>,
    /// The comments of each topic (by topic id), grown as batches arrive.
    pub threads: HashMap<i64, CommentThread>,
    /// The streamed topic list for a path (e.g. `latest.json`), grown
    /// incrementally as topics arrive.
    pub streams: HashMap<ShareRef<str>, TopicStream>,
    /// Maps a topic's `category_id` to its display name.
    pub categories: HashMap<i64, ShareRef<str>>,
    /// The full category (color, slug, ...) by id, for drawing its pill.
    pub category_details: HashMap<i64, Rc<Category>>,
    /// How the app theme is chosen (follows the system by default).
    pub theme_mode: ThemeMode,
    /// The tabs of the bar of a narrow window.
    pub navbar: NavbarTabs,
    /// The windows open beside the app's own; see [`Windows`].
    pub windows: Windows,
    /// What the search bar holds. The bar lives in the shell and the results in
    /// the page, so the store is where they meet.
    /// The signed-in user's username. Nobody can sign in yet, so it is `None`.
    pub current_user: Option<String>,
    /// Counts the sign-ins that changed who the app is signed in as. A request
    /// remembers the count it started under and drops its answer when it no
    /// longer matches, so a list fetched as a guest never lands in the lists of
    /// the member who signed in while it was on its way.
    pub session_epoch: u64,
    pub search_term: String,
    pub search_mode: SearchMode,
    /// What each search found, by its (trimmed) term and mode.
    pub searches: HashMap<(String, SearchMode), SearchResults>,
    /// The most recent results of each mode, for the page to keep showing while
    /// the next search for another term is on its way.
    pub last_searches: HashMap<SearchMode, SearchResults>,
}

/// The comments loaded so far for one topic, in post order.
#[derive(Clone, PartialEq, Default)]
pub struct CommentThread {
    pub comments: Vec<Rc<Comment>>,
    pub status: StreamStatus,
    /// The ids of the posts the topic document did not carry, in topic order.
    /// Set once by [`AppStore::start_comments`]; shared so a clone is a bump.
    pending: Rc<[i64]>,
    /// How many of `pending` have been handed out to be fetched.
    cursor: usize,
}

/// The streaming state of one topic-list request.
#[derive(Clone, PartialEq, Default)]
pub struct TopicStream {
    pub topics: Vec<Shared<Topic>>,
    pub status: StreamStatus,
    /// The `?page=` to ask for next; `0` is the path as it stands.
    next_page: u32,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum StreamStatus {
    #[default]
    Loading,
    /// More is available but nothing has asked for it yet; a comment thread or
    /// topic list waits here until the reader scrolls near its end.
    Idle,
    Done,
    Error(String),
}

impl AppStore {
    /// Records `username` as the signed-in member and drops every list streamed
    /// before, since the forum answers a member differently from a guest: some
    /// lists (`new.json`) are only theirs to see, and the rest carry what they
    /// have and have not read. The lists are streamed again when next shown.
    ///
    /// Signing in as who is already in changes nothing, so a restored session
    /// that turns up after a sign-in by hand does not throw its lists away.
    pub fn sign_in(&mut self, username: String) {
        if self.current_user.as_deref() == Some(username.as_str()) {
            return;
        }
        self.current_user = Some(username);
        // A window opened to sign in from has done its work.
        if self.windows.dialog == Some(DialogPage::Login) {
            self.windows.dialog = None;
        }
        self.streams.clear();
        self.session_epoch += 1;
    }

    /// Forgets who the app is signed in as, and every list streamed as them: the
    /// forum answers a guest differently, see [`AppStore::sign_in`].
    pub fn sign_out(&mut self) {
        if self.current_user.take().is_some() {
            // Whose profile it showed is gone.
            self.windows.profile = false;
            self.streams.clear();
            self.session_epoch += 1;
        }
    }

    pub async fn fetch_latest(&mut self) {
        let store = Api::get("latest.json").await.unwrap();
        self.root = store;
    }

    pub async fn fetch_categories() -> Result<CategoryRoot, String> {
        Api::get::<CategoryRoot>("categories.json")
            .await
            .map_err(|e| e.to_string())
    }

    pub fn cached_post(&self, id: i64) -> Option<PostRoot> {
        self.posts.get(&id).cloned()
    }

    pub fn cache_post(&mut self, id: i64, post: PostRoot) {
        self.posts.insert(id, post);
    }

    pub fn cached_actions(&self, username: &str, filter: ActionFilter) -> Option<Vec<UserAction>> {
        self.actions.get(&(username.to_string(), filter)).cloned()
    }

    pub fn cache_actions(
        &mut self,
        username: &str,
        filter: ActionFilter,
        actions: Vec<UserAction>,
    ) {
        self.actions.insert((username.to_string(), filter), actions);
    }

    pub fn cached_search(&self, term: &str, mode: SearchMode) -> Option<SearchResults> {
        self.searches.get(&(term.to_string(), mode)).cloned()
    }

    pub fn cache_search(&mut self, term: &str, mode: SearchMode, results: SearchResults) {
        self.last_searches.insert(mode, results.clone());
        self.searches.insert((term.to_string(), mode), results);
    }

    pub fn last_search(&self, mode: SearchMode) -> Option<SearchResults> {
        self.last_searches.get(&mode).cloned()
    }

    pub fn cached_profile(&self, username: &str) -> Option<UserProfile> {
        self.profiles.get(username).cloned()
    }

    pub fn cache_profile(&mut self, username: &str, profile: UserProfile) {
        self.profiles.insert(username.to_string(), profile);
    }

    pub fn comment_thread(&self, topic_id: i64) -> CommentThread {
        self.threads.get(&topic_id).cloned().unwrap_or_default()
    }

    /// Adds `comments` to a topic's thread, skipping any already present so a
    /// repeated load cannot duplicate them.
    pub fn push_comments(&mut self, topic_id: i64, comments: Vec<Comment>) {
        let thread = self.threads.entry(topic_id).or_default();
        for comment in comments {
            if !thread
                .comments
                .iter()
                .any(|known| known.post_number == comment.post_number)
            {
                thread.comments.push(Rc::new(comment));
            }
        }
        thread.comments.sort_by_key(|comment| comment.post_number);
    }

    /// Whether a thread was ever opened for the topic.
    pub fn has_comment_thread(&self, topic_id: i64) -> bool {
        self.threads.contains_key(&topic_id)
    }

    /// Whether a chunk is waiting to be asked for: the thread is idle with ids
    /// left. A plain lookup, for callers that ask on every scroll frame.
    pub fn wants_more_comments(&self, topic_id: i64) -> bool {
        self.threads.get(&topic_id).is_some_and(|thread| {
            thread.status == StreamStatus::Idle && thread.cursor < thread.pending.len()
        })
    }

    /// Opens a topic's thread with the posts its document carried and the ids of
    /// those it did not (`missing`, in topic order), which are fetched later, a
    /// chunk at a time, as the reader nears the end.
    ///
    /// A thread that is already open is left as it is: the loader runs again
    /// whenever the page is rebuilt, and must not rewind a thread underway.
    pub fn start_comments(&mut self, topic_id: i64, initial: Vec<Comment>, missing: Vec<i64>) {
        if self.threads.contains_key(&topic_id) {
            return;
        }
        let status = if missing.is_empty() {
            StreamStatus::Done
        } else {
            StreamStatus::Idle
        };
        self.threads.insert(
            topic_id,
            CommentThread {
                comments: Vec::new(),
                status,
                pending: missing.into(),
                cursor: 0,
            },
        );
        self.push_comments(topic_id, initial);
    }

    /// Hands out the next up to `limit` ids still to be fetched and marks the
    /// thread `Loading`, or returns `None` when a request is already on its way,
    /// nothing is left, or the thread failed. Only one request is ever in
    /// flight, so it is safe to ask on every scroll event.
    pub fn claim_comment_chunk(&mut self, topic_id: i64, limit: usize) -> Option<Vec<i64>> {
        let thread = self.threads.get_mut(&topic_id)?;
        if thread.status != StreamStatus::Idle {
            return None;
        }
        let end = (thread.cursor + limit).min(thread.pending.len());
        let chunk = thread.pending[thread.cursor..end].to_vec();
        if chunk.is_empty() {
            return None;
        }
        thread.cursor = end;
        thread.status = StreamStatus::Loading;
        Some(chunk)
    }

    /// Marks the chunk just pushed as in: `Idle` while ids remain, `Done` after
    /// the last. Leaves a failed thread failed.
    pub fn settle_comments(&mut self, topic_id: i64) {
        let Some(thread) = self.threads.get_mut(&topic_id) else {
            return;
        };
        if matches!(thread.status, StreamStatus::Error(_)) {
            return;
        }
        thread.status = if thread.cursor < thread.pending.len() {
            StreamStatus::Idle
        } else {
            StreamStatus::Done
        };
    }

    pub fn fail_comments(&mut self, topic_id: i64, error: String) {
        self.threads.entry(topic_id).or_default().status = StreamStatus::Error(error);
    }

    pub fn topic_stream(&self, path: &str) -> TopicStream {
        self.streams.get(path).cloned().unwrap_or_default()
    }

    /// The title of a topic that has been listed, whichever list it came from.
    ///
    /// A post page uses it to show the title before the post itself has loaded,
    /// so the page opening out of a card is recognisably that card's topic.
    pub fn topic_title(&self, id: i64) -> Option<String> {
        self.streams
            .values()
            .flat_map(|stream| stream.topics.iter())
            .find(|topic| topic.id == id)
            .map(|topic| topic.title.clone())
    }

    /// The likes and replies a post's action bar shows: from the post once it
    /// has loaded, else from its topic card, else nothing.
    pub fn post_counts(&self, id: i64) -> (i64, i64) {
        if let Some(post) = self.posts.get(&id) {
            return (post.like_count, post.reply_count);
        }
        self.streams
            .values()
            .flat_map(|stream| stream.topics.iter())
            .find(|topic| topic.id == id)
            .map_or((0, 0), |topic| {
                (topic.like_count, topic.posts_count.saturating_sub(1))
            })
    }

    /// Adds a topic to a list. One already in it is left alone: the forum's pages
    /// shift as topics are bumped, so the next page may repeat the last one's end.
    pub fn push_stream_topic(&mut self, path: ShareRef<str>, topic: Topic) {
        let stream = self.streams.entry(path).or_default();
        if stream.topics.iter().any(|known| known.id == topic.id) {
            return;
        }
        stream.topics.push(Shared::new(topic));
    }

    /// Marks the page just streamed as in: `Idle` when the forum has `more`
    /// after it, `Done` when it was the last.
    pub fn finish_stream(&mut self, path: ShareRef<str>, page: u32, more: bool) {
        let stream = self.streams.entry(path).or_default();
        stream.next_page = stream.next_page.max(page + 1);
        stream.status = if more {
            StreamStatus::Idle
        } else {
            StreamStatus::Done
        };
    }

    pub fn fail_stream(&mut self, path: ShareRef<str>, error: String) {
        self.streams.entry(path).or_default().status = StreamStatus::Error(error);
    }

    /// Whether a page of a list is waiting to be asked for. A plain lookup, for
    /// callers that ask on every scroll frame.
    pub fn wants_more_topics(&self, path: &str) -> bool {
        self.streams
            .get(path)
            .is_some_and(|stream| stream.status == StreamStatus::Idle)
    }

    /// Hands out the next page of a list and marks it `Loading`, or returns
    /// `None` when a request is already on its way, the list is at its end, or it
    /// failed. Only one request is ever in flight, so it is safe to ask on every
    /// scroll event.
    pub fn claim_topic_page(&mut self, path: &str) -> Option<u32> {
        let stream = self.streams.get_mut(path)?;
        if stream.status != StreamStatus::Idle {
            return None;
        }
        stream.status = StreamStatus::Loading;
        Some(stream.next_page)
    }

    pub fn category_name(&self, id: i64) -> Option<ShareRef<str>> {
        self.categories.get(&id).cloned()
    }

    pub fn category(&self, id: i64) -> Option<Rc<Category>> {
        self.category_details.get(&id).cloned()
    }

    pub fn set_theme_mode(&mut self, mode: ThemeMode) {
        self.theme_mode = mode;
    }

    pub fn cache_categories(&mut self, categories: &[Rc<Category>]) {
        self.categories = categories
            .into_iter()
            .map(|category| (category.id, category.project(|item| item.name.as_str())))
            .collect();
        self.category_details = categories
            .iter()
            .map(|category| (category.id, category.clone()))
            .collect();
    }

    pub async fn stream_posts(&mut self) {
        let body = match crate::api::session::client()
            .get("https://users.rust-lang.org/latest.json")
            .send()
            .await
        {
            Ok(res) => res.text().await,
            Err(_) => return,
        };

        let store = match body {
            Ok(body) => serde_json::from_str::<Root>(&body).unwrap(),
            Err(_) => return,
        };

        self.root = store;
    }
}

#[cfg(test)]
mod test_store {
    use crate::models::Root;
    use crate::store::AppStore;

    #[tokio::test]
    async fn test_fetch_latest() {
        let mut store = AppStore::default();
        store.fetch_latest().await;

        println!("{:?}", store.root);
        assert_ne!(store.root, Root::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topic(id: i64, title: &str) -> Topic {
        Topic {
            id,
            title: title.to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn a_topic_title_is_found_in_any_stream() {
        let mut store = AppStore::default();
        store.push_stream_topic(ShareRef::from_static("latest.json"), topic(1, "First"));
        store.push_stream_topic(ShareRef::from_static("hot.json"), topic(2, "Second"));

        assert_eq!(store.topic_title(1).as_deref(), Some("First"));
        assert_eq!(store.topic_title(2).as_deref(), Some("Second"));
    }

    #[test]
    fn a_topic_that_was_never_listed_has_no_title() {
        let mut store = AppStore::default();
        assert_eq!(store.topic_title(7), None);

        store.push_stream_topic(ShareRef::from_static("latest.json"), topic(1, "First"));
        assert_eq!(store.topic_title(7), None);
    }

    #[test]
    fn a_list_hands_out_its_next_page_once_and_only_while_more_remains() {
        let mut store = AppStore::default();
        let path = ShareRef::from_static("latest.json");
        store.push_stream_topic(path.clone(), topic(1, "First"));

        // The first page is still streaming: nothing to ask for.
        assert!(!store.wants_more_topics("latest.json"));
        assert_eq!(store.claim_topic_page("latest.json"), None);

        store.finish_stream(path.clone(), 0, true);
        assert!(store.wants_more_topics("latest.json"));
        // A scroll event per frame must not start the same request twice.
        assert_eq!(store.claim_topic_page("latest.json"), Some(1));
        assert_eq!(store.claim_topic_page("latest.json"), None);
        assert_eq!(store.topic_stream("latest.json").status, StreamStatus::Loading);

        store.finish_stream(path.clone(), 1, true);
        assert_eq!(store.claim_topic_page("latest.json"), Some(2));

        store.finish_stream(path, 2, false);
        assert_eq!(store.topic_stream("latest.json").status, StreamStatus::Done);
        assert_eq!(store.claim_topic_page("latest.json"), None);
    }

    #[test]
    fn a_topic_the_next_page_repeats_is_listed_once() {
        let mut store = AppStore::default();
        let path = ShareRef::from_static("latest.json");
        store.push_stream_topic(path.clone(), topic(1, "First"));
        store.push_stream_topic(path, topic(1, "First"));

        assert_eq!(store.topic_stream("latest.json").topics.len(), 1);
    }

    #[test]
    fn a_failed_page_is_not_asked_for_again_by_scrolling() {
        let mut store = AppStore::default();
        let path = ShareRef::from_static("latest.json");
        store.push_stream_topic(path.clone(), topic(1, "First"));
        store.finish_stream(path.clone(), 0, true);
        store.claim_topic_page("latest.json");
        store.fail_stream(path, "offline".to_owned());

        assert!(!store.wants_more_topics("latest.json"));
        assert_eq!(store.claim_topic_page("latest.json"), None);
    }

    #[test]
    fn signing_in_drops_the_lists_fetched_as_a_guest() {
        let mut store = AppStore::default();
        let path = ShareRef::from_static("new.json");
        store.push_stream_topic(path.clone(), topic(1, "First"));
        store.fail_stream(path, "Required to Login".to_owned());
        let before = store.session_epoch;

        store.sign_in("domma".to_owned());

        assert_eq!(store.current_user.as_deref(), Some("domma"));
        assert!(store.topic_stream("new.json").topics.is_empty());
        assert_eq!(store.topic_stream("new.json").status, StreamStatus::Loading);
        assert_ne!(store.session_epoch, before);
    }

    #[test]
    fn signing_in_as_who_is_already_in_changes_nothing() {
        let mut store = AppStore::default();
        store.sign_in("domma".to_owned());
        store.push_stream_topic(ShareRef::from_static("latest.json"), topic(1, "First"));
        let epoch = store.session_epoch;

        store.sign_in("domma".to_owned());

        assert_eq!(store.topic_stream("latest.json").topics.len(), 1);
        assert_eq!(store.session_epoch, epoch);
    }

    #[test]
    fn signing_out_drops_the_member_and_the_lists_streamed_as_them() {
        let mut store = AppStore::default();
        store.sign_in("domma".to_owned());
        store.push_stream_topic(ShareRef::from_static("new.json"), topic(1, "First"));
        let epoch = store.session_epoch;

        store.sign_out();

        assert_eq!(store.current_user, None);
        assert!(store.topic_stream("new.json").topics.is_empty());
        assert_ne!(store.session_epoch, epoch);

        // Nobody to sign out changes nothing.
        let epoch = store.session_epoch;
        store.sign_out();
        assert_eq!(store.session_epoch, epoch);
    }

    #[test]
    fn a_search_is_remembered_by_its_term_and_mode() {
        use crate::models::search::{SearchMode, SearchResults};
        let mut store = AppStore::default();
        store.cache_search("apple", SearchMode::Blog, SearchResults::Blog(vec![]));

        assert!(store.cached_search("apple", SearchMode::Blog).is_some());
        assert!(store.cached_search("apple", SearchMode::User).is_none());
        assert!(store.cached_search("pear", SearchMode::Blog).is_none());
    }

    #[test]
    fn the_last_results_of_a_mode_stay_to_be_shown_while_the_next_term_loads() {
        use crate::models::search::{BlogHit, SearchMode, SearchResults};
        let mut store = AppStore::default();
        assert!(store.last_search(SearchMode::Blog).is_none());

        let hits = SearchResults::Blog(vec![BlogHit::default()]);
        store.cache_search("apple", SearchMode::Blog, hits.clone());
        assert_eq!(store.last_search(SearchMode::Blog), Some(hits));
        assert!(store.last_search(SearchMode::User).is_none());
    }

    fn reply(post_number: i64) -> Comment {
        Comment {
            post_number,
            reply_to: None,
            username: format!("user{post_number}"),
            avatar_url: String::new(),
            created_at: String::new(),
            like_count: 0,
            markdown: Rc::from(""),
        }
    }

    #[test]
    fn a_thread_with_posts_left_waits_until_they_are_asked_for() {
        let mut store = AppStore::default();
        store.start_comments(1, vec![reply(2)], vec![3, 4, 5]);

        let thread = store.comment_thread(1);
        assert_eq!(thread.comments.len(), 1);
        assert_eq!(thread.status, StreamStatus::Idle);
    }

    #[test]
    fn a_thread_with_nothing_left_is_done() {
        let mut store = AppStore::default();
        store.start_comments(1, vec![reply(2)], vec![]);

        assert_eq!(store.comment_thread(1).status, StreamStatus::Done);
        assert_eq!(store.claim_comment_chunk(1, 20), None);
    }

    #[test]
    fn asking_for_more_hands_out_one_chunk_and_blocks_a_second_ask() {
        let mut store = AppStore::default();
        store.start_comments(1, vec![], vec![3, 4, 5]);

        assert_eq!(store.claim_comment_chunk(1, 2), Some(vec![3, 4]));
        assert_eq!(store.comment_thread(1).status, StreamStatus::Loading);
        // A scroll event per frame must not start the same request twice.
        assert_eq!(store.claim_comment_chunk(1, 2), None);
    }

    #[test]
    fn settling_a_chunk_allows_the_next_until_the_last_is_in() {
        let mut store = AppStore::default();
        store.start_comments(1, vec![], vec![3, 4, 5]);

        assert_eq!(store.claim_comment_chunk(1, 2), Some(vec![3, 4]));
        store.push_comments(1, vec![reply(3), reply(4)]);
        store.settle_comments(1);
        assert_eq!(store.comment_thread(1).status, StreamStatus::Idle);

        assert_eq!(store.claim_comment_chunk(1, 2), Some(vec![5]));
        store.push_comments(1, vec![reply(5)]);
        store.settle_comments(1);
        assert_eq!(store.comment_thread(1).status, StreamStatus::Done);
        assert_eq!(store.claim_comment_chunk(1, 2), None);
    }

    #[test]
    fn a_failed_chunk_ends_the_requests_for_that_thread() {
        let mut store = AppStore::default();
        store.start_comments(1, vec![], vec![3, 4]);
        assert_eq!(store.claim_comment_chunk(1, 2), Some(vec![3, 4]));

        store.fail_comments(1, "offline".to_owned());

        assert_eq!(store.claim_comment_chunk(1, 2), None);
        assert_eq!(
            store.comment_thread(1).status,
            StreamStatus::Error("offline".to_owned())
        );
    }

    #[test]
    fn asking_about_a_topic_with_no_thread_hands_out_nothing() {
        let mut store = AppStore::default();
        assert_eq!(store.claim_comment_chunk(9, 20), None);
    }

    #[test]
    fn starting_a_thread_again_does_not_reset_one_that_is_underway() {
        let mut store = AppStore::default();
        store.start_comments(1, vec![reply(2)], vec![3, 4, 5]);
        assert_eq!(store.claim_comment_chunk(1, 2), Some(vec![3, 4]));

        // The page is rebuilt and its loader runs again with the same topic.
        store.start_comments(1, vec![reply(2)], vec![3, 4, 5]);

        assert_eq!(store.comment_thread(1).status, StreamStatus::Loading);
        assert_eq!(store.comment_thread(1).comments.len(), 1);
    }
}
