mod inline {
    mod inline {
    }
    mod file;
    mod folder;
}
mod file;
mod folder;

#[path = "path_attr/test.rs"]
mod path;

#[path = "inline_path_value"]
mod inline_path {
    mod file;
    mod folder;

    #[path = "path_attr_inside_value.rs"]
    mod path_attr_inside;
}