fn main() -> Result<(), wdk_build::ConfigError> {
    unsafe { std::env::set_var("CARGO_CFG_TARGET_FEATURE", "crt-static") };
    wdk_build::configure_wdk_binary_build()
}
