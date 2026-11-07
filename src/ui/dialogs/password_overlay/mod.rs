mod builder;

use gtk4::prelude::*;
use std::rc::Rc;

pub type VerifyCallback = Rc<dyn Fn(String)>;
pub type VerifyCallbackCell = Rc<std::cell::RefCell<Option<VerifyCallback>>>;

pub struct PasswordOverlayWidget {
    container: gtk4::Box,
    err_label: gtk4::Label,
    entry: gtk4::PasswordEntry,
    ok_btn: gtk4::Button,
    cancel_btn: gtk4::Button,
    current_verify_fn: VerifyCallbackCell,
}

impl Default for PasswordOverlayWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordOverlayWidget {
    pub fn new() -> Self {
        let elements = builder::build();
        let current_verify_fn: VerifyCallbackCell = Rc::new(std::cell::RefCell::new(None));

        let current_fn_c = current_verify_fn.clone();
        let entry_c = elements.entry.clone();
        let submit_action = move || {
            let pwd = entry_c.text().to_string();
            if let Some(ref verify) = *current_fn_c.borrow() {
                verify(pwd);
            }
        };

        let submit_c1 = Rc::new(submit_action);
        let submit_c2 = submit_c1.clone();

        elements.entry.connect_activate(move |_| {
            submit_c1();
        });

        elements.ok_btn.connect_clicked(move |_| {
            submit_c2();
        });

        let container_c = elements.container.clone();
        elements.cancel_btn.connect_clicked(move |_| {
            container_c.set_visible(false);
        });

        Self {
            container: elements.container,
            err_label: elements.err_label,
            entry: elements.entry,
            ok_btn: elements.ok_btn,
            cancel_btn: elements.cancel_btn,
            current_verify_fn,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn is_visible(&self) -> bool {
        self.container.is_visible()
    }

    pub fn prompt<F>(&self, verify_fn: F)
    where
        F: Fn(String, Rc<dyn Fn(bool, &str)>) + 'static,
    {
        self.err_label.set_visible(false);
        self.err_label.set_text("");
        self.entry.set_text("");
        self.container.set_visible(true);
        self.entry.grab_focus();

        let verify_fn_rc = Rc::new(verify_fn);
        let container_c = self.container.clone();
        let err_label_c = self.err_label.clone();
        let entry_c = self.entry.clone();
        let ok_btn_c = self.ok_btn.clone();
        let cancel_btn_c = self.cancel_btn.clone();

        let verify_wrapper = move |pwd: String| {
            if pwd.is_empty() {
                err_label_c.set_text("Please enter a password");
                err_label_c.set_visible(true);
                return;
            }

            ok_btn_c.set_sensitive(false);
            cancel_btn_c.set_sensitive(false);

            let cnt_done = container_c.clone();
            let err_done = err_label_c.clone();
            let ent_done = entry_c.clone();
            let ok_done = ok_btn_c.clone();
            let cancel_done = cancel_btn_c.clone();

            let on_done = Rc::new(move |success: bool, err_msg: &str| {
                ok_done.set_sensitive(true);
                cancel_done.set_sensitive(true);

                if success {
                    cnt_done.set_visible(false);
                } else {
                    err_done.set_text(if err_msg.is_empty() {
                        "❌ Incorrect password. Please try again."
                    } else {
                        err_msg
                    });
                    err_done.set_visible(true);
                    ent_done.grab_focus();
                    ent_done.select_region(0, -1);
                }
            });

            verify_fn_rc(pwd, on_done);
        };

        *self.current_verify_fn.borrow_mut() = Some(Rc::new(verify_wrapper));
    }
}
