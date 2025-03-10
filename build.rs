fn main() {
    #[cfg(windows)]
    {
        let _ = embed_resource::compile("assets/windows/halloy.rc", embed_resource::NONE);
        windows_exe_info::versioninfo::link_cargo_env();
    }
}
