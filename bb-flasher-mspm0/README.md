# bb-flasher-mspm0

Local notes for reproducing the MSPM0 I2C flashing session used for the
JK-Embedded audio board.

## Current target assumption

The Linux I2C backend is hardcoded to use address `0x48` in `src/i2c.rs`.

That matched the existing code path used during this bring-up session, but it is
not a general MSPM0 discovery mechanism. On the JK-Embedded board, `0x48` is
also documented as a DAC address, so this path should be treated as
board-specific until the hardware side is verified more directly.

## Local behavior confirmed on hardware

The JK-Embedded board now responds reliably enough in ROM BSL mode to prove the
full pre-flash handshake:

- `CONNECTION` returns `0x00`
- `GET_DEVICE_INFO` returns `0x00` followed by a valid 32-byte payload
- `UNLOCK` returns `0x00`
- `MASS_ERASE` returns a 9-byte core response packet, not a lone `0x00`

The important nuance is that erase/program success is not consistently reported
as a one-byte ACK on this board. `src/bsl.rs` now accepts either:

- a single-byte `0x00` ACK, or
- a valid `CORE_MESSAGE` response packet beginning with `0x08`

`src/helpers.rs` still skips standalone verification because that path is not
yet proven reliable on this board.

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

Enter ROM BSL on the BeagleY-AI host using line names:

```console
gpioset -C mspm0-bsl GPIO25=1
gpioset -C mspm0-reset -t 200ms,0 GPIO24=0
```

Flash the JK-Embedded test image while `GPIO25` is still held high:

```console
./target/debug/bb-imager-cli flash zepto --no-verify \
  /home/beagle/jkembedded-audio-board/build/mspm0-zephyr/zephyr/zephyr.hex \
  /dev/i2c-1
```

The successful run observed during bring-up emitted:

- `[1] Preparing`
- `[3] Verifying`

Return the MCU to normal boot:

```console
gpioset -C mspm0-boot-low -p 100ms GPIO25=0
gpioset -C mspm0-reset-normal -t 200ms,0 GPIO24=0
```

Final expected line state:

- `gpioget --numeric GPIO24 GPIO25` -> `1 0`

## Useful raw sanity checks

With the MSPM0 forced into BSL mode, this connection packet produced an ACK:

```console
i2ctransfer -y 1 w8@0x48 0x80 0x01 0x00 0x12 0x3a 0x61 0x44 0xde r1
```

Expected response:

- `0x00`

`GET_DEVICE_INFO` also works as separate write/read transactions:

```console
i2ctransfer -y 1 w8@0x48 0x80 0x01 0x00 0x19 0xb2 0xb8 0x96 0x49
i2ctransfer -y 1 r1@0x48
i2ctransfer -y 1 r32@0x48
```

Expected responses:

- first read: `0x00`
- second read: a 32-byte response beginning with `0x08 0x19 0x00 0x31`

`UNLOCK` also returns `0x00`:

```console
i2ctransfer -y 1 \
  w40@0x48 \
  0x80 0x21 0x00 0x21 \
  0xff 0xff 0xff 0xff 0xff 0xff 0xff 0xff \
  0xff 0xff 0xff 0xff 0xff 0xff 0xff 0xff \
  0xff 0xff 0xff 0xff 0xff 0xff 0xff 0xff \
  0xff 0xff 0xff 0xff 0xff 0xff 0xff 0xff \
  0xff 0xff 0xff 0xff 0x02 0xaa 0xf0 0x3d
i2ctransfer -y 1 r1@0x48
```

`MASS_ERASE` is where the protocol differs from the original flasher
assumption. The board returns a core response packet:

```console
i2ctransfer -y 1 w8@0x48 0x80 0x01 0x00 0x15 0x99 0xf4 0x20 0x40
i2ctransfer -y 1 r1@0x48
i2ctransfer -y 1 r8@0x48
```

Expected responses:

- first read: `0x08`
- second read: `0x02 0x00 0x3b 0x00 0x38 0x02 0x94 0x82`

## Next steps

- Replace the hardcoded target address with an explicit CLI option or board
  profile.
- Restore strong verification once standalone verification is understood on this
  board.
- Confirm whether `0x48` is truly the MSPM0 BSL path on this board or an
  accidental overlap with another device.
- Add logging that prints whether a command succeeded by one-byte ACK or by
  `CORE_MESSAGE` response packet.
