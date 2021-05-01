use crate::settings::Settings;
use log::error;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;

pub struct ExtTools {
    allowed: bool,
    library_root: String,
    edit_programs: HashMap<String, String>,
}

impl ExtTools {
    fn check_allowed(&self) -> bool {
        if !self.allowed {
            error!("Opening disabled! Set `allow_external_tools` in server config to `true`.");
            return false;
        }
        return true;
    }

    pub fn open_library_root(&self) {
        if !self.check_allowed() {
            return;
        }

        if let Err(e) = open::that(&self.library_root) {
            error!("Cannot open library root: {:?}", e);
        }
    }

    pub fn edit_file<P: AsRef<Path> + Debug>(&self, path: P) {
        if !self.check_allowed() {
            return;
        }

        let extension = path.as_ref().extension().map(|x| x.to_str()).flatten();

        match extension {
            None => {} // no extension or cannot convert str
            Some(extension) => match self.edit_programs.get(extension) {
                None => {
                    if let Err(e) = open::that(path.as_ref()) {
                        error!("Cannot edit file {:?}: {:?}", path, e);
                    }
                }
                Some(program) => {
                    if let Err(e) = open::with(path.as_ref(), program) {
                        error!("Cannot edit file {:?}: {:?}", path, e);
                    }
                }
            },
        }
    }
}

pub fn create_ext_tools(settings: &Settings) -> Arc<ExtTools> {
    let library = ExtTools {
        allowed: settings.allow_external_tools,
        library_root: settings.library_root.clone(),
        edit_programs: settings
            .external_tools
            .as_ref()
            .map(|ref x| convert_to_edit_programs(x))
            .unwrap_or_else(|| HashMap::new()),
    };

    Arc::new(library)
}

fn convert_to_edit_programs(
    external_tools: &HashMap<String, Vec<String>>,
) -> HashMap<String, String> {
    let mut result: HashMap<String, String> = HashMap::new();

    for (tool, extensions) in external_tools.iter() {
        for extension in extensions.iter() {
            match result.entry(extension.to_string()) {
                Entry::Occupied(t) => {
                    panic!("Invalid configuration: The extension {:?} has at least two programs {:?} and {:?} specified as editors! For each extensions you must provide exactly one program.", extension, tool, t.get());
                }
                Entry::Vacant(t) => {
                    t.insert(tool.to_string());
                }
            }
        }
    }

    result
}
