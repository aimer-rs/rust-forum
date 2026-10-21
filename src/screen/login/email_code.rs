use super::flow::{self, CODE_LENGTH, Step};
use super::{form_page, paragraph};
use crate::api::session::{self, Verified};
use crate::components::auth_button::{filled_button, text_button};
use crate::components::auth_field::AuthField;
use crate::router::AppRouter;
use crate::store::AppStore;
use crate::theme::ForumTheme;
use aimer::input::TextEditingController;
use aimer::input::input::InputType;
use aimer::router::NavigatorController;
use aimer::{
    AnyWidget, BuildContext, FutureSnapshotExt, Icons, ProviderHandle,
    SnapshotTask, State, StateUpdater, StatefulWidget, Widget,
};
use aimer::style::Theme;
use std::rc::Rc;

/// What to do once the forum has signed the member in, with their username.
type SignedIn = Rc<dyn Fn(String)>;

/// The page to sign in by a one-time code emailed to the member: first the
/// address to send it to, then the code that came, then, for an account with
/// 2-step verification, its passcode.
///
/// It is a page of its own, one level in from the sign-in page, so that it
/// slides in over it and the back button and the edge swipe slide it away again.
#[derive(Clone, StatefulWidget)]
pub struct EmailCodeScreen;

pub struct EmailCodeState {
    step: Step,
    email: TextEditingController,
    code: TextEditingController,
    passcode: TextEditingController,
    /// What the last press came to, shown under the form.
    message: Option<String>,
    /// The request to the forum that is out, if one is. Holding it is what keeps
    /// the request alive: leaving the page drops it, and with it the request.
    request: Option<SnapshotTask<(), String>>,
    updater: StateUpdater<Self>,
}

impl StatefulWidget for EmailCodeScreen {
    type State = EmailCodeState;

    fn create_state(self) -> Self::State {
        EmailCodeState {
            step: Step::EmailAddress,
            email: TextEditingController::new(),
            code: TextEditingController::new(),
            passcode: TextEditingController::new(),
            message: None,
            request: None,
            updater: StateUpdater::new(),
        }
    }
}

impl EmailCodeState {
    fn go(&mut self, step: Step) {
        self.step = step;
        self.message = None;
        self.request = None;
        self.code.clear();
        self.passcode.clear();
    }

    /// Whether a request to the forum is out.
    fn sending(&self) -> bool {
        self.request.is_some()
    }

    /// Asks the forum for a code for the address typed, if it could be one.
    fn send_code(&mut self) {
        let email = self.email.value().text().to_owned();
        match flow::submit_email(&email) {
            Ok(Step::EmailCode { email }) => self.request_code(email),
            Ok(_) => {}
            Err(note) => self.message = Some(note.into()),
        }
    }

    /// Asks again for a code for the address the last one went to.
    fn resend_code(&mut self) {
        if let Step::EmailCode { email } = self.step.clone() {
            self.request_code(email);
        }
    }

    /// Sends the request for a code for `email`, unless one is already out.
    ///
    /// The page hears of the answer through [`Self::sent`].
    fn request_code(&mut self, email: String) {
        if self.sending() {
            return;
        }
        self.message = None;
        let updater = self.updater;
        self.request = Some(
            async move {
                let outcome = session::request_login_code(&email).await;
                let answer = outcome.clone();
                updater.set_state(move |state| state.sent(email, answer));
                outcome
            }
            .waitme(),
        );
    }

    /// What the forum answered to the request for a code for `email`.
    fn sent(&mut self, email: String, outcome: Result<(), String>) {
        self.request = None;
        match outcome {
            Ok(()) => {
                let again = matches!(self.step, Step::EmailCode { .. });
                self.go(Step::EmailCode { email });
                if again {
                    self.message = Some("We sent you a new code.".into());
                }
            }
            Err(note) => self.message = Some(note),
        }
    }

    /// Checks the code typed, if it is the shape of one.
    fn verify_code(&mut self, signed_in: SignedIn) {
        let Step::EmailCode { email } = self.step.clone() else {
            return;
        };
        match flow::submit_code(&self.code.value().text()) {
            Ok(code) => self.verify(email, code, None, signed_in),
            Err(note) => self.message = Some(note.into()),
        }
    }

    /// Checks the code again, this time with the passcode typed.
    fn verify_passcode(&mut self, signed_in: SignedIn) {
        let Step::Passcode { email, code } = self.step.clone() else {
            return;
        };
        match flow::submit_passcode(&self.passcode.value().text()) {
            Ok(passcode) => self.verify(email, code, Some(passcode), signed_in),
            Err(note) => self.message = Some(note.into()),
        }
    }

    /// Sends the code, with a passcode if there is one, unless a request is out.
    ///
    /// The page hears of a passcode being asked for or a code refused through
    /// [`Self::verified`]. A member who is in is handed to `signed_in` and the
    /// page is left to it: it goes away, and the request with it.
    fn verify(&mut self, email: String, code: String, passcode: Option<String>, signed_in: SignedIn) {
        if self.sending() {
            return;
        }
        self.message = None;
        let updater = self.updater;
        self.request = Some(
            async move {
                let outcome = session::verify_login_code(&email, &code, passcode.as_deref()).await;
                match &outcome {
                    Ok(Verified::SignedIn(username)) => signed_in(username.clone()),
                    _ => {
                        let answer = outcome.clone();
                        updater.set_state(move |state| state.verified(email, code, answer));
                    }
                }
                outcome.map(|_| ())
            }
            .waitme(),
        );
    }

    /// What the forum answered to checking the code of `email`.
    fn verified(&mut self, email: String, code: String, outcome: Result<Verified, String>) {
        self.request = None;
        match outcome {
            Ok(Verified::NeedsPasscode) => {
                self.go(Step::Passcode { email, code });
            }
            Ok(Verified::SignedIn(_)) => {}
            Err(note) => self.message = Some(note),
        }
    }

    /// The widgets of the form for the step the page is on.
    fn body(&self, theme: ForumTheme, signed_in: SignedIn) -> Vec<AnyWidget> {
        let updater = self.updater;
        let sending = self.sending();
        let (on_submit, on_press) = (signed_in.clone(), signed_in.clone());
        match &self.step {
            Step::EmailAddress => vec![
                AuthField::new(self.email.clone(), Icons::MAIL, "Email address")
                    .input_type(InputType::Email)
                    .on_submitted(move || updater.set_state(|state| state.send_code()))
                    .boxed(),
                filled_button(theme, if sending { "Sending…" } else { "Send code" }, move || {
                    updater.set_state(|state| state.send_code())
                }),
            ],
            Step::EmailCode { email } => vec![
                paragraph(theme, &format!("Enter the {CODE_LENGTH}-digit code sent to {email}."), 16, 200),
                AuthField::new(self.code.clone(), Icons::KEY_ROUND, "6-digit code")
                    .input_type(InputType::Number)
                    .max_length(CODE_LENGTH)
                    .on_submitted(move || {
                        let signed_in = on_submit.clone();
                        updater.set_state(move |state| state.verify_code(signed_in))
                    })
                    .boxed(),
                filled_button(theme, if sending { "Verifying…" } else { "Verify and sign in" }, move || {
                    let signed_in = on_press.clone();
                    updater.set_state(move |state| state.verify_code(signed_in))
                }),
                text_button(
                    if sending { "Sending…" } else { "Send a new code" },
                    theme.primary_color(),
                    15,
                    move || updater.set_state(|state| state.resend_code()),
                ),
                text_button("Use another address", theme.primary_color(), 15, move || {
                    updater.set_state(|state| state.go(Step::EmailAddress))
                }),
            ],
            Step::Passcode { .. } => vec![
                paragraph(
                    theme,
                    "This account uses 2-step verification. Enter the passcode from your authenticator app, or a backup code.",
                    16,
                    200,
                ),
                AuthField::new(self.passcode.clone(), Icons::LOCK, "Passcode")
                    .on_submitted(move || {
                        let signed_in = on_submit.clone();
                        updater.set_state(move |state| state.verify_passcode(signed_in))
                    })
                    .boxed(),
                filled_button(theme, if sending { "Verifying…" } else { "Verify and sign in" }, move || {
                    let signed_in = on_press.clone();
                    updater.set_state(move |state| state.verify_passcode(signed_in))
                }),
                text_button("Use another address", theme.primary_color(), 15, move || {
                    updater.set_state(|state| state.go(Step::EmailAddress))
                }),
            ],
        }
    }
}

impl State<EmailCodeScreen> for EmailCodeState {
    fn init_state(&mut self, updater: StateUpdater<Self>) {
        self.updater = updater;
    }

    fn adopt_config_from(&mut self, _new: Self) {}

    fn build(&self, ctx: &BuildContext) -> impl Widget {
        let theme = *ForumTheme::of(ctx);
        let store = ProviderHandle::<AppStore>::of(ctx);
        let navigator = NavigatorController::<AppRouter>::of(ctx);
        let signed_in: SignedIn = Rc::new(move |username| {
            store.update(|store| store.sign_in(username));
            navigator.set_route(AppRouter::Account);
        });
        form_page(
            ctx,
            theme,
            "Email code",
            "We'll email you a code to sign in with.",
            self.body(theme, signed_in),
            self.message.as_deref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> EmailCodeState {
        EmailCodeScreen.create_state()
    }

    #[test]
    fn it_starts_by_asking_for_the_address() {
        let state = state();
        assert_eq!(state.step, Step::EmailAddress);
        assert!(state.message.is_none());
    }

    #[test]
    fn a_bad_address_stays_on_the_address_step_with_a_note() {
        let mut state = state();
        state.email.set_text("not an address");
        state.send_code();
        assert_eq!(state.step, Step::EmailAddress);
        assert!(state.message.is_some());
    }

    #[test]
    fn a_good_address_sends_a_request_and_waits_for_the_answer_on_the_same_step() {
        let mut state = state();
        state.message = Some("old".into());
        state.email.set_text(" ferris@rust-lang.org ");
        state.send_code();
        assert!(state.sending());
        assert_eq!(state.step, Step::EmailAddress);
        assert!(state.message.is_none());
    }

    #[test]
    fn a_second_press_while_one_is_out_sends_nothing_more() {
        let mut state = state();
        state.email.set_text("ferris@rust-lang.org");
        state.send_code();
        state.message = Some("kept".into());
        state.send_code();
        assert_eq!(state.message.as_deref(), Some("kept"));
    }

    #[test]
    fn a_taken_request_moves_on_to_the_code_and_clears_the_note() {
        let mut state = state();
        state.message = Some("old".into());
        state.sent("ferris@rust-lang.org".into(), Ok(()));
        assert_eq!(state.step, Step::EmailCode { email: "ferris@rust-lang.org".into() });
        assert!(state.message.is_none() && !state.sending());
    }

    #[test]
    fn a_refused_request_stays_on_the_address_with_the_forums_note() {
        let mut state = state();
        state.email.set_text("ferris@rust-lang.org");
        state.send_code();
        state.sent("ferris@rust-lang.org".into(), Err("Slow down.".into()));
        assert_eq!(state.step, Step::EmailAddress);
        assert_eq!(state.message.as_deref(), Some("Slow down."));
        assert!(!state.sending());
    }

    #[test]
    fn asking_again_for_a_code_says_so_when_it_is_sent() {
        let mut state = state();
        state.step = Step::EmailCode { email: "a@b.co".into() };
        state.resend_code();
        assert!(state.sending());
        state.sent("a@b.co".into(), Ok(()));
        assert_eq!(state.step, Step::EmailCode { email: "a@b.co".into() });
        assert_eq!(state.message.as_deref(), Some("We sent you a new code."));
    }

    fn nobody() -> SignedIn {
        Rc::new(|_| {})
    }

    #[test]
    fn a_code_that_is_not_six_digits_is_refused_before_anything_else() {
        let mut state = state();
        state.step = Step::EmailCode { email: "a@b.co".into() };
        state.code.set_text("12ab");
        state.verify_code(nobody());
        assert_eq!(state.message.as_deref(), Some("The code is 6 digits."));
        assert!(!state.sending());
    }

    #[test]
    fn a_code_of_six_digits_is_sent_to_the_forum() {
        let mut state = state();
        state.step = Step::EmailCode { email: "a@b.co".into() };
        state.code.set_text("941 233");
        state.verify_code(nobody());
        assert!(state.sending());
        assert!(state.message.is_none());
    }

    #[test]
    fn a_code_the_forum_wants_a_passcode_for_moves_on_to_the_passcode() {
        let mut state = state();
        state.step = Step::EmailCode { email: "a@b.co".into() };
        state.message = Some("old".into());
        state.verified("a@b.co".into(), "941233".into(), Ok(Verified::NeedsPasscode));
        assert_eq!(state.step, Step::Passcode { email: "a@b.co".into(), code: "941233".into() });
        assert!(state.message.is_none() && !state.sending());
    }

    #[test]
    fn a_refused_code_stays_on_the_step_with_the_forums_note() {
        let mut state = state();
        state.step = Step::EmailCode { email: "a@b.co".into() };
        state.verified("a@b.co".into(), "941233".into(), Err("Wrong code.".into()));
        assert_eq!(state.step, Step::EmailCode { email: "a@b.co".into() });
        assert_eq!(state.message.as_deref(), Some("Wrong code."));
    }

    #[test]
    fn a_passcode_is_asked_for_before_anything_is_sent() {
        let mut state = state();
        state.step = Step::Passcode { email: "a@b.co".into(), code: "941233".into() };
        state.verify_passcode(nobody());
        assert_eq!(state.message.as_deref(), Some("Enter your passcode."));
        assert!(!state.sending());
        state.passcode.set_text("123 456");
        state.verify_passcode(nobody());
        assert!(state.sending());
    }

    #[test]
    fn a_press_on_the_wrong_step_sends_nothing() {
        let mut state = state();
        state.code.set_text("941233");
        state.verify_code(nobody());
        state.verify_passcode(nobody());
        assert!(!state.sending());
    }

    #[test]
    fn going_back_to_the_address_forgets_the_code_and_the_note() {
        let mut state = state();
        state.step = Step::EmailCode { email: "a@b.co".into() };
        state.code.set_text("123456");
        state.message = Some("note".into());
        state.go(Step::EmailAddress);
        assert_eq!(state.step, Step::EmailAddress);
        assert!(state.message.is_none() && state.code.value().text().is_empty());
    }
}
