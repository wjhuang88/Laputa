use std::fs;
use rlua::Lua;

#[test]
fn test_load_lua() {
    let paths = fs::read_dir("./deploy").unwrap();
    for path in paths {
        let path_entry = path.unwrap().path();
        println!("Script file name: {}", path_entry.display());
        match fs::read(path_entry) {
            Ok(p) => Lua::new().context(|lua| {
                lua.load(p.as_slice()).exec().unwrap();
            }),
            Err(e) => panic!(e)
        };
    }
}