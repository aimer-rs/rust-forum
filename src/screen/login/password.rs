use super::form_page;
use crate::components::auth_button::filled_button;
use crate::components::auth_field::AuthField;
use crate::theme::ForumTheme;
use aimer::input::TextEditingController;
use aimer::input::input::InputType;
use aimer::style::Theme;
use aimer::{BuildContext, Icons, State, StateUpdater, StatefulWidget, Widget};

/// The page to sign in by a username (or email) and a password.
///
/// It is a page of its own, one level in from the sign-in page, so that it slides
/// in over it and the back button and the edge swipe slide it away again.
#[derive(Clone, StatefulWidget)]
pub struct PasswordScreen;

pub struct PasswordState {
    identity: TextEditingController,
    password: TextEditingController,
    /// What the last press came to, shown under the form.
    message: Option<String>,
    updater: StateUpdater<Self>,
}

impl StatefulWidget for PasswordScreen {
    type State = PasswordState;

    fn create_state(self) -> Self::State {
        PasswordState {
            identity: TextEditingController::new(),
            password: TextEditingController::new(),
            message: None,
            updater: StateUpdater::new(),
        }
    }
}

impl PasswordState {
    fn sign_in(&mut self) {
        self.message = Some(
            if self.identity.value().text().trim().is_empty() {
                "Enter your username or email."
            } else if self.password.value().text().is_empty() {
                "Enter your password."
            } else {
                "Signing in with a password isn't available yet."
            }
            .into(),
        );
    }
}

impl State<PasswordScreen> for PasswordState {
    fn init_state(&mut self, updater: StateUpdater<Self>) {
        self.updater = updater;
    }

    fn adopt_config_from(&mut self, _new: Self) {}

    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let updater = self.updater;
        let submit = move || updater.set_state(|state| state.sign_in());

        let body = vec![
            AuthField::new(self.identity.clone(), Icons::USER, "Username or email")
                .on_submitted(submit)
                .boxed(),
            AuthField::new(self.password.clone(), Icons::LOCK, "Password")
                .input_type(InputType::Password)
                .on_submitted(submit)
                .boxed(),
            filled_button(theme, "Log in", submit),
        ];
        form_page(ctx, theme, "Log in", "Welcome back to the Rust forum.", body, self.message.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_asks_for_what_is_missing_before_saying_it_is_not_available() {
        let mut state = PasswordScreen.create_state();
        state.sign_in();
        assert_eq!(state.message.as_deref(), Some("Enter your username or email."));
        state.identity.set_text("ferris");
        state.sign_in();
        assert_eq!(state.message.as_deref(), Some("Enter your password."));
        state.password.set_text("hunter2");
        state.sign_in();
        assert!(state.message.as_deref().is_some_and(|m| m.contains("isn't available yet")));
    }
}
