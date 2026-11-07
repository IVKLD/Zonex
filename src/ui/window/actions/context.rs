use crate::archive::ArchiveManager;
use crate::ui::dialogs::PasswordOverlayWidget;
use crate::ui::file_view::FileView;
use crate::utils::TempCache;
use libadwaita::ApplicationWindow;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub type ToastCallback = Rc<dyn Fn(&str)>;
pub type LoadingCallback = Rc<dyn Fn(bool, &str)>;
pub type OpenCallback = Rc<dyn Fn(PathBuf)>;

pub struct ActionsContext {
    pub window: ApplicationWindow,
    pub manager: Arc<Mutex<ArchiveManager>>,
    pub temp_cache: Rc<TempCache>,
    pub file_view: Rc<FileView>,
    pub password_overlay: Rc<PasswordOverlayWidget>,
    pub show_toast: ToastCallback,
    pub set_loading: LoadingCallback,
    pub do_open: OpenCallback,
}
