extern crate lit;

#[allow(dead_code)]
#[cfg(test)]
mod tests{
    use std::env;
    use std::path::PathBuf;

    fn path_to_string(path: &PathBuf) -> String {
        path.to_str().unwrap().to_string()
    }

    fn get_bin_dir() -> PathBuf {
        env::current_exe().ok().map(|mut path: PathBuf| {
            path.pop();
            path.pop();
            path
        }).unwrap()
    }

    fn get_calcc() -> String {
        path_to_string(&get_bin_dir().join(
            format!("{}{}", "calcc", env::consts::EXE_SUFFIX)
        ))
    }

    // Disabled -- #[test]
    fn lit() {
        lit::run::tests(
            lit::event_handler::Default::new(),
            |config: &mut lit::config::Config| {
                config.add_search_path("tests/lit-rust");
                config.add_extension("calc");
                config.constants.insert("calcc".to_owned(), get_calcc());
            }
        ).expect("Lit tests failed") ;
    }
}
