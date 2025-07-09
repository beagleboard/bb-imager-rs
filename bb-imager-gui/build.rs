fn main() {
    embed_resource::compile(
        "assets/packages/windows/gui-manifest.rc",
        embed_resource::NONE,
    )
    .manifest_required()
    .unwrap();
}
