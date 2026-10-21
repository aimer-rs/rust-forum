//! What the member has chosen for how the app looks and is laid out, and keeping
//! it from one launch to the next.
//!
//! None of it is secret, so it lives in the app's plain storage; the sign-in is
//! what the vault is for, see [`crate::api::session`].

use crate::router::AppRouter;
use crate::store::AppStore;
use aimer::storage::{FileStorage, Storage};
use aimer::style::ThemeMode;
use aimer::{ProviderHandle, Venus};
use std::future::Future;

const APP_ID: &str = "dev.aimers.rustforum";
const NAMESPACE: &str = "preferences";
const THEME_KEY: &str = "theme-mode";
const NAVBAR_KEY: &str = "navbar-tabs";

/// The fewest tabs the bar of a narrow window keeps: one tab is no choice.
pub const MIN_TABS: usize = 2;

/// A place the tab bar of a narrow window can send the app to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavTab {
    Latest,
    New,
    Hot,
    Search,
    Account,
}

impl NavTab {
    /// Every tab there is, in the order the bar shows those it has.
    pub const ALL: [NavTab; 5] = [Self::Latest, Self::New, Self::Hot, Self::Search, Self::Account];

    pub fn label(self) -> &'static str {
        match self {
            Self::Latest => "Latest",
            Self::New => "New",
            Self::Hot => "Hot",
            Self::Search => "Search",
            Self::Account => "Account",
        }
    }

    pub fn route(self) -> AppRouter {
        match self {
            Self::Latest => AppRouter::Latest,
            Self::New => AppRouter::New,
            Self::Hot => AppRouter::Hot,
            Self::Search => AppRouter::Search,
            Self::Account => AppRouter::Account,
        }
    }

    /// Whether the bar cannot do without the tab. The account page is where the
    /// settings are opened from, so a bar without it would leave no way to them.
    pub fn is_required(self) -> bool {
        self == Self::Account
    }

    fn key(self) -> &'static str {
        match self {
            Self::Latest => "latest",
            Self::New => "new",
            Self::Hot => "hot",
            Self::Search => "search",
            Self::Account => "account",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.key() == key)
    }
}

/// The tabs the bar of a narrow window shows: at least [`MIN_TABS`], always
/// including the required ones, each once and in the order of [`NavTab::ALL`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavbarTabs(Vec<NavTab>);

impl Default for NavbarTabs {
    fn default() -> Self {
        Self(NavTab::ALL.to_vec())
    }
}

impl NavbarTabs {
    /// The bar that has `tabs`, set right where they cannot stand: the required
    /// tabs are added, and so are others in the default order until there are
    /// enough.
    pub fn new(tabs: impl IntoIterator<Item = NavTab>) -> Self {
        let wanted: Vec<NavTab> = tabs.into_iter().collect();
        let chosen = |tab: &NavTab| tab.is_required() || wanted.contains(tab);
        let mut kept: Vec<NavTab> = NavTab::ALL.into_iter().filter(|tab| chosen(tab)).collect();
        for tab in NavTab::ALL {
            if kept.len() >= MIN_TABS {
                break;
            }
            if !kept.contains(&tab) {
                kept.push(tab);
            }
        }
        // Back in the order the bar shows them.
        Self(NavTab::ALL.into_iter().filter(|tab| kept.contains(tab)).collect())
    }

    pub fn tabs(&self) -> &[NavTab] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn contains(&self, tab: NavTab) -> bool {
        self.0.contains(&tab)
    }

    /// Whether `tab` can be switched: a tab that is in can go unless it is
    /// required or the bar would be left too short, and one that is out can
    /// always come in.
    pub fn can_toggle(&self, tab: NavTab) -> bool {
        !self.contains(tab) || (!tab.is_required() && self.len() > MIN_TABS)
    }

    /// The bar with `tab` switched, or this one when it cannot be.
    pub fn toggled(&self, tab: NavTab) -> Self {
        if !self.can_toggle(tab) {
            return self.clone();
        }
        if self.contains(tab) {
            Self::new(self.0.iter().copied().filter(|other| *other != tab))
        } else {
            Self::new(self.0.iter().copied().chain([tab]))
        }
    }

    fn encode(&self) -> String {
        self.0.iter().map(|tab| tab.key()).collect::<Vec<_>>().join(",")
    }

    /// The bar saved as `text`; a name this version doesn't know is skipped, so a
    /// bar saved by a newer one still opens.
    fn decode(text: &str) -> Option<Self> {
        let tabs: Vec<NavTab> = text.split(',').filter_map(|key| NavTab::from_key(key.trim())).collect();
        (!tabs.is_empty()).then(|| Self::new(tabs))
    }
}

fn theme_key(mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::System => "system",
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
    }
}

fn theme_from_key(key: &str) -> Option<ThemeMode> {
    [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark]
        .into_iter()
        .find(|mode| theme_key(*mode) == key)
}

/// What was saved by an earlier launch, for what the member chose.
#[derive(Default)]
pub struct Saved {
    pub theme_mode: Option<ThemeMode>,
    pub navbar: Option<NavbarTabs>,
}

async fn read(storage: &FileStorage, key: &str) -> Option<String> {
    let bytes = storage.read(NAMESPACE, key).await.ok()??;
    String::from_utf8(bytes).ok()
}

/// What an earlier launch saved. A choice that cannot be read is as good as one
/// that was never made.
pub async fn load() -> Saved {
    let Ok(storage) = FileStorage::for_app(APP_ID) else {
        return Saved::default();
    };
    Saved {
        theme_mode: read(&storage, THEME_KEY).await.as_deref().and_then(theme_from_key),
        navbar: read(&storage, NAVBAR_KEY).await.as_deref().and_then(NavbarTabs::decode),
    }
}

async fn write(key: &'static str, value: String) {
    if let Ok(storage) = FileStorage::for_app(APP_ID) {
        // A choice that cannot be kept still holds until the app is closed.
        let _ = storage.write(NAMESPACE, key, value.as_bytes()).await;
    }
}

/// Runs `save` on the task queue of the UI thread, where storage is awaited.
///
/// The tests keep nothing: they would write over the choices of whoever runs them.
fn in_background(save: impl Future<Output = ()> + 'static) {
    if cfg!(test) {
        return;
    }
    if let Some(venus) = Venus::current() {
        venus.spawn(save);
    }
}

/// Switches the app to `mode` and keeps it.
pub fn set_theme_mode(store: &ProviderHandle<AppStore>, mode: ThemeMode) {
    store.update(move |store| store.set_theme_mode(mode));
    in_background(write(THEME_KEY, theme_key(mode).to_owned()));
}

/// Gives the tab bar of a narrow window `tabs` and keeps them.
pub fn set_navbar(store: &ProviderHandle<AppStore>, tabs: NavbarTabs) {
    let saved = tabs.encode();
    store.update(move |store| store.navbar = tabs);
    in_background(write(NAVBAR_KEY, saved));
}

#[cfg(test)]
mod tests {
    use super::*;
    use aimer::router::Route;

    fn tabs(bar: &NavbarTabs) -> Vec<&'static str> {
        bar.tabs().iter().map(|tab| tab.label()).collect()
    }

    #[test]
    fn the_bar_starts_with_every_tab() {
        assert_eq!(tabs(&NavbarTabs::default()), ["Latest", "New", "Hot", "Search", "Account"]);
    }

    #[test]
    fn a_bar_is_set_right_where_it_cannot_stand() {
        // The account tab is added, a tab given twice is kept once and the order is the bar's.
        assert_eq!(tabs(&NavbarTabs::new([NavTab::Hot, NavTab::Latest, NavTab::Hot])), ["Latest", "Hot", "Account"]);
        // Too short a bar is filled up from the front.
        assert_eq!(tabs(&NavbarTabs::new([])), ["Latest", "Account"]);
        assert_eq!(tabs(&NavbarTabs::new([NavTab::Account])), ["Latest", "Account"]);
    }

    #[test]
    fn a_tab_can_be_taken_out_and_put_back() {
        let bar = NavbarTabs::default().toggled(NavTab::New);
        assert_eq!(tabs(&bar), ["Latest", "Hot", "Search", "Account"]);
        assert_eq!(tabs(&bar.toggled(NavTab::New)), ["Latest", "New", "Hot", "Search", "Account"]);
    }

    #[test]
    fn the_account_tab_and_the_last_other_tab_stay() {
        let bar = NavbarTabs::default();
        assert!(!bar.can_toggle(NavTab::Account));
        assert_eq!(bar.toggled(NavTab::Account), bar);

        let two = NavbarTabs::new([NavTab::Search]);
        assert_eq!(tabs(&two), ["Search", "Account"]);
        assert!(!two.can_toggle(NavTab::Search));
        assert_eq!(two.toggled(NavTab::Search), two);
        assert!(two.can_toggle(NavTab::Hot));
    }

    #[test]
    fn a_bar_survives_being_saved() {
        let bar = NavbarTabs::default().toggled(NavTab::Hot).toggled(NavTab::New);
        assert_eq!(bar.encode(), "latest,search,account");
        assert_eq!(NavbarTabs::decode(&bar.encode()), Some(bar));
    }

    #[test]
    fn a_saved_bar_with_names_from_another_version_still_opens() {
        assert_eq!(
            NavbarTabs::decode("latest, categories ,hot").map(|bar| tabs(&bar)),
            Some(vec!["Latest", "Hot", "Account"])
        );
        assert_eq!(NavbarTabs::decode(""), None);
        assert_eq!(NavbarTabs::decode("categories"), None);
    }

    #[test]
    fn each_theme_mode_survives_being_saved() {
        for mode in [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark] {
            assert_eq!(theme_from_key(theme_key(mode)), Some(mode));
        }
        assert_eq!(theme_from_key("sepia"), None);
    }

    #[test]
    fn each_tab_goes_to_its_own_route() {
        let routes: Vec<_> = NavTab::ALL.iter().map(|tab| tab.route().format()).collect();
        assert_eq!(routes, ["/", "/new", "/hot", "/search", "/account"]);
    }
}
