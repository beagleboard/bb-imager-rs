pub(crate) fn check_token(
    cancel: Option<&bb_helper::cancel::CancellationToken>,
) -> crate::Result<()> {
    match cancel {
        Some(x) if x.is_cancelled() => Err(crate::Error::Aborted),
        _ => Ok(()),
    }
}

pub(crate) fn is_dfu_device<U: rusb::UsbContext>(x: &rusb::Device<U>) -> bool {
    if let Ok(cfg_desc) = x.active_config_descriptor() {
        for intf in cfg_desc.interfaces() {
            for desc in intf.descriptors() {
                if desc.class_code() == 0xfe && desc.sub_class_code() == 1 {
                    return true;
                }
            }
        }
    }

    false
}
