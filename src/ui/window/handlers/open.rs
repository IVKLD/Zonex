use super::open_worker::{apply_open_success, prompt_password_retry, run_open_worker};
use crate::archive::ArchiveManager;
use crate::ui::dialogs::PasswordOverlayWidget;
use crate::ui::file_view::FileView;
use crate::ui::window::actions::{LoadingCallback, OpenCallback, ToastCallback};
use crate::utils::TempCache;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub fn create_open_handler(
    manager: Arc<Mutex<ArchiveManager>>,
    temp_cache: Rc<TempCache>,
    file_view: Rc<FileView>,
    pwd_overlay: Rc<PasswordOverlayWidget>,
    show_toast: ToastCallback,
    set_loading: LoadingCallback,
) -> OpenCallback {
    let do_open_path = move |path: PathBuf, password: Option<String>| {
        set_loading(true, "Opening archive (mounting FUSE...)");
        temp_cache.clear_stage();

        let mgr_base = manager.clone();
        let fv = file_view.clone();
        let sl = set_loading.clone();
        let toast = show_toast.clone();
        let overlay = pwd_overlay.clone();

        glib::MainContext::default().spawn_local(async move {
            let res = run_open_worker(mgr_base.clone(), path, password).await;
            if res.success {
                apply_open_success(res, &fv, &*sl, &*toast);
            } else {
                sl(false, "Password required or invalid format");
                if !overlay.is_visible() {
                    prompt_password_retry(
                        overlay.clone(),
                        mgr_base.clone(),
                        fv.clone(),
                        sl.clone(),
                        toast.clone(),
                        res.path,
                    );
                }
            }
        });
    };

    let do_open_rc = Rc::new(do_open_path);
    Rc::new(move |p| do_open_rc(p, None))
}
