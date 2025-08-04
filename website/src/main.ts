import './style.scss'
import Alpine from 'alpinejs'

const BB_IMAGER_VERSION = import.meta.env.VITE_BB_IMAGER_VERSION
const LATEST_BUILDS = [
  {
    name: "Windows (Installer)",
    packages: [
      { name: "x64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-gui_${BB_IMAGER_VERSION}_x64_en-US.msi` }
    ]
  },
  {
    name: "Windows (Portable)",
    packages: [
      { name: "x64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-gui_${BB_IMAGER_VERSION}_x86_64.exe` },
      { name: "ARM64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-gui_${BB_IMAGER_VERSION}_aarch64.exe` }
    ]
  },
  {
    name: "MacOS (DMG)",
    packages: [
      { name: "x64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/BeagleBoard.Imaging.Utility_${BB_IMAGER_VERSION}_x64.dmg` },
      { name: "ARM64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/BeagleBoard.Imaging.Utility_${BB_IMAGER_VERSION}_aarch64.dmg` }
    ]
  },
  {
    name: "Linux (AppImage)",
    packages: [
      { name: "x64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-gui_${BB_IMAGER_VERSION}_x86_64.AppImage` },
      { name: "ARM64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-gui_${BB_IMAGER_VERSION}_aarch64.AppImage` }
    ]
  }
]
const LINUX_BUILDS_MORE = [
  {
    name: "Flatpak Package",
    packages: [{ name: "Flathub", url: "https://flathub.org/apps/org.beagleboard.imagingutility" }]
  },
  {
    name: "Debian Linux Package",
    packages: [
      { name: "x64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-gui_${BB_IMAGER_VERSION}_amd64.deb` },
      { name: "ARM64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-gui_${BB_IMAGER_VERSION}_arm64.deb` },
      { name: "ARM", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-gui_${BB_IMAGER_VERSION}_armhf.deb` },
    ]
  },
  {
    name: "Generic Linux Package",
    packages: [
      { name: "x64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-gui_${BB_IMAGER_VERSION}_x86_64.tar.gz` },
      { name: "ARM64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-gui_${BB_IMAGER_VERSION}_aarch64.tar.gz` },
      { name: "ARM", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-gui_${BB_IMAGER_VERSION}_armhf.tar.gz` },
    ]
  }
]
const CLI_BUILDS = [
  {
    name: "Debian Linux Package",
    packages: [
      { name: "x64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-cli_${BB_IMAGER_VERSION}_amd64.deb` },
      { name: "ARM64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-cli_${BB_IMAGER_VERSION}_arm64.deb` },
      { name: "ARM", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-cli_${BB_IMAGER_VERSION}_armhf.deb` },
    ]
  },
  {
    name: "Generic Linux Package",
    packages: [
      { name: "x64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-cli_${BB_IMAGER_VERSION}_x86_64.tar.gz` },
      { name: "ARM64", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-cli_${BB_IMAGER_VERSION}_aarch64.tar.gz` },
      { name: "ARM", url: `https://github.com/beagleboard/bb-imager-rs/releases/download/v${BB_IMAGER_VERSION}/bb-imager-cli_${BB_IMAGER_VERSION}_armhf.tar.gz` },
    ]
  }
]

Alpine.data('latest_builds', () => ({
  latest_packages: LATEST_BUILDS,
  linux_builds_more: LINUX_BUILDS_MORE,
  cli_builds: CLI_BUILDS
}))

Alpine.start()