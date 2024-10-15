use crate::pom::Pom;
use std::env;

mod pom;

fn main() {
    let args: Vec<String> = env::args().collect();
    let skip_group = &args[1];
    let dir = &args[2];

    check_pom_versions(dir, skip_group);
}

fn check_pom_versions(root_dir: &str, skip_group: &str) {
    let mut pom = Pom::from_file(root_dir, skip_group);

    let parents = &pom.fetch_parents();

    pom.fill_missing_properties(parents);
    pom.fetch_latest_versions();
}
