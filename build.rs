extern crate embed_resource;

fn main() {
    #[cfg(windows)]
    embed_resource::compile("assets/windows/halloy.rc", embed_resource::NONE);
}
