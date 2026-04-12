# bb-flasher-mspm0

Local notes for reproducing the MSPM0 I2C flashing session used for the
JK-Embedded audio board.

## Current target assumption

The Linux I2C backend is hardcoded to use address `0x48` in `src/i2c.rs`.

That matched the existing code path used during this bring-up session, but it is
not a general MSPM0 discovery mechanism. On the JK-Embedded board, `0x48` is
also documented as a DAC address, so this path should be treated as
board-specific until the hardware side is verified more directly.

## Local workaround applied

The board acknowledged BSL command packets but did not return reliable response
payloads for `GET_DEVICE_INFO`, `UNLOCK`, erase/program core responses, or the
standalone verification CRC.

To keep the flashing path moving for this board, the local changes in
`src/bsl.rs` and `src/helpers.rs` do the following:

- fall back to a fixed BSL max buffer size when `GET_DEVICE_INFO` parsing fails
- trust the I2C ACK for `unlock`, `mass_erase`, and `program_data`
- skip the pre-flash and post-flash standalone verification path

This is intentionally a narrow bring-up workaround, not a robust upstream-ready
solution.

## Reproduce on the BeagleY-AI host

Build the CLI:

```console
cd /home/beagle/bb-imager-rs
. "$HOME/.cargo/env"
cargo build -p bb-imager-cli --no-default-features --features zepto_i2c
```

List I2C destinations:

```console
./target/debug/bb-imager-cli list-destinations zepto
```

Expected output on the bring-up host included:

- `/dev/i2c-1`
- `/dev/i2c-2`
- `/dev/i2c-3`

Flash the JK-Embedded test image:

```console
./target/debug/bb-imager-cli flash zepto --no-verify \
  /home/beagle/jkembedded-audio-board/build/mspm0-zephyr/zephyr/zephyr.hex \
  /dev/i2c-1
```

The successful run observed during bring-up emitted:

- `[1] Preparing`
- `[3] Verifying`

## Useful raw sanity checks

With the MSPM0 forced into BSL mode, this connection packet produced an ACK:

```console
i2ctransfer -y 1 w8@0x48 0x80 0x01 0x00 0x12 0x3a 0x61 0x44 0xde r1
```

Expected response:

- `0x00`

The follow-up body reads were unreliable on this board, which is why the local
workaround exists.

## Next steps

- Replace the hardcoded target address with an explicit CLI option or board
  profile.
- Restore strong verification once the response-body failure is understood.
- Confirm whether `0x48` is truly the MSPM0 BSL path on this board or an
  accidental overlap with another device.
- Add logging that prints whether the tool is using the normal verified path or
  the degraded ACK-only workaround.
