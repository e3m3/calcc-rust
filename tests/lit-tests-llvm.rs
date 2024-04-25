#[cfg(test)]
mod tests{
    use std::env;
    use std::path::Path;
    use std::path::PathBuf;
    use std::process::Command;

    fn path_to_string(path: &Path) -> String {
        String::from(path.to_str().unwrap())
    }

    fn pathbuf_to_string(path: &PathBuf) -> String {
        path.to_str().unwrap().to_string()
    }

    fn get_bin_dir() -> PathBuf {
        env::current_exe().ok().map(|mut path: PathBuf| {
            path.pop();
            path.pop();
            path
        }).unwrap()
    }

    fn get_tests_dir() -> PathBuf {
        env::current_exe().ok().map(|mut path: PathBuf| {
            path.pop();
            path.pop();
            path.pop();
            path.pop();
            path.push("tests");
            path
        }).unwrap()
    }

    fn get_shell() -> String {
        String::from(if cfg!(target_os = "linux") {"bash"} else {"cmd"})
    }

    fn get_lit() -> &'static str {
        if cfg!(target_os = "linux") {
            "/usr/bin/lit"
        } else {
            eprintln!("OS not supported");
            assert!(false);
            ""
        }
    }

    #[test]
    fn lit() {
        if !cfg!(target_os = "linux") {
            eprintln!("OS not yet supported.");
            assert!(false);
        }

        let calcc_dir: PathBuf = get_bin_dir();
        let lit_bin: &Path = Path::new(get_lit());
        let shell: String = get_shell();
        let tests_dir: PathBuf = get_tests_dir();
        let lit_dir: PathBuf = tests_dir.join("lit-llvm");
        let cfg_path: PathBuf = lit_dir.join("lit.cfg");

        assert!(calcc_dir.is_dir());
        assert!(lit_bin.is_file());
        assert!(lit_dir.is_dir());
        assert!(cfg_path.is_file());

        let calcc_dir_str: String = pathbuf_to_string(&calcc_dir);
        let lit_dir_str: String = pathbuf_to_string(&lit_dir);
        let lit_bin_str: String = path_to_string(&lit_bin);

        let env_path_str: String = [
            calcc_dir_str,
        ].join(":");

        let lit_args: String = [
            "--config-prefix=lit",
            "--order=lexical",
            "--show-all",
            "--workers=4",
            format!("--path={}", env_path_str).as_str(),
        ].join(" ");

        println!("Processing tests in directory: {}", pathbuf_to_string(&lit_dir));
        let output = Command::new(&shell)
            .arg("-c")
            .arg(format!("{} {} {}", lit_bin_str, lit_args, lit_dir_str))
            .output()
            .expect("Failed lit tests");
        let stderr: &[u8] = output.stderr.as_slice();
        let stdout: &[u8] = output.stdout.as_slice();

        println!();
        eprintln!("Lit stderr:\n{}", std::str::from_utf8(stderr).unwrap());
        println!("Lit stdout:\n{}", std::str::from_utf8(stdout).unwrap());

        assert!(stderr.len() == 0);
    }
}
