mod add_files;
mod context;
mod delete;
mod entry;
mod extract;
mod integrity;
mod new_archive;
mod password;
mod selection;

pub use context::{ActionsContext, LoadingCallback, OpenCallback, ToastCallback};

pub fn register(ctx: &ActionsContext) {
    selection::register(ctx);
    integrity::register(ctx);
    delete::register(ctx);
    extract::register(ctx);
    add_files::register(ctx);
    entry::register(ctx);
    new_archive::register(ctx);
    password::register(ctx);
}
