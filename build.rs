fn main() {
    let string = std::fs::read_to_string(".env").unwrap();
    let string = string.split("\n").collect::<Vec<_>>();
    for i in string.iter() {
        let equal = i.find('=').unwrap();
        let before = &i[0..equal];
        let before = before.replace(" ", "");
        let after = &i[equal..i.len()];
        let after = after.replace("= ", "");
        println!("cargo:rustc-env={before}={after}");
    }
}
