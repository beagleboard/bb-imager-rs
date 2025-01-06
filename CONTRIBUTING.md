# Development

## Dependencies

The following dependencies are required to work with the codebase.

1. [git-lfs](https://git-lfs.com/): Since this is a Git LFS repository, a lot of weird errors regarding missing assets will come up cloning the repository without installing `git-lfs`.
2. [rust](https://www.rust-lang.org/): I personally use [rustup](https://rustup.rs/) to manage Rust versions, but development can be done without Rustup as well since everything builds with stable Rust.

### Platform Dependencies

#### Ubuntu and other Debian-based Linux distro

```
apt-get install -y --no-install-recommends libudev-dev
```

## Run

**Note:** When trying to actually flash hardware or doing benchmarking, ensure that you are running a release build (`--release`).

- GUI

```
cargo run -p bb-imager-gui
```

- CLI

```
cargo run -p bb-imager-cli
```

# Submitting PRs

This document exists to ensure some conventions and standards for all the PRs. They are not rigid and can be evaluated on a case-by-case basis, but as a general rule, they should be followed while contributing. It takes inspiration from [Submitting patches: the essential guide to getting your code into the kernel](https://docs.kernel.org/process/submitting-patches.html), although it differs since the project does not need to maintain as stringent requirements as Linux Kernel.

This document also allows me to point people to the document rather than trying to explain the problems with each PR. 

## Describe your changes

Describe your problem. Whether your patch is a one-line bug fix or 5000 lines of a new feature, there must be an underlying problem that motivated you to do this work. Convince the reviewer that there is a problem worth fixing and that it makes sense for them to read past the first paragraph.

Once the problem is established, please describe what you are actually doing about it in technical detail. It’s important to describe the change in plain English for the reviewer to verify that the code is behaving as you intend it to.

## Separate your changes

Separate each logical change into a separate commit or PRs.

It is fine to group a series of commits in a single PR if they aim to tackle a single problem (like a Gitlab issue), but remember, shorter PRs are easier to review and thus would be merged faster.

## Style-check your changes

Remember to run [clippy](https://doc.rust-lang.org/clippy/) to check the style of the code. I do not plan to deviate from the standard Rust style in any way, so [rustfmt](https://rust-lang.github.io/rustfmt/) and [clippy](https://doc.rust-lang.org/clippy/) are your friends.

## Sign your work - the Developer’s Certificate of Origin

Improves tracking of who did what. While being mostly a remnant of the fact that Linux kernel development happens in a mailing list, I quite like the simple `Signed-off-by` entry since sometimes it is difficult to preserve the authors on abandoned work.

The sign-off is a simple line at the end of the explanation for the commit, which certifies that you wrote it or otherwise have the right to pass it on as an open-source patch. The rules are pretty simple: if you can certify the following:

### Developer’s Certificate of Origin 1.1

By making a contribution to this project, I certify that:

1. The contribution was created in whole or in part by me, and I have the right to submit it under the open-source license indicated in the file; or

2. The contribution is based upon previous work that, to the best of my knowledge, is covered under an appropriate open-source license, and I have the right under that license to submit that work with modifications, whether created in whole or in part by me, under the same open source license (unless I am permitted to submit under a different license), as indicated in the file; or

3. The contribution was provided directly to me by some other person who certified (a), (b), or (c), and I have not modified it.

4. I understand and agree that this project and the contribution are public and that a record of the contribution (including all personal information I submit with it, including my sign-off) is maintained indefinitely and may be redistributed consistent with this project or the open source license(s) involved.

Then you just add a line saying:

```
Signed-off-by: Ayush Singh <ayush@beagleboard.org>
```

using a known identity (sorry, no anonymous contributions.) This will be done for you automatically if you use `git commit -s`. Reverts should also include “Signed-off-by”. `git revert -s` does that for you.

## Changelog Trailers

This repository uses [GitLab Changelog Entries](https://docs.gitlab.com/ee/development/changelog.html) to generate release Changelog. The Changelog trailer accepts the following values:

- added: New feature
- fixed: Bug fix
- changed: Feature change
- deprecated: New deprecation
- removed: Feature removal
- security: Security fix
- performance: Performance improvement
- other: Other 

An example of a Git commit to include in the Changelog is the following: 

```
Update git vendor to GitLab

Now that we are using gitaly to compile git, the git version isn't known
from the manifest, instead, we are getting the gitaly version. Update our
vendor field to be `gitlab` to avoid cve matching old versions.

Changelog: changed
Signed-off-by: XYZ <abc@email.com>
```

The changelog entries are only required in cases of user-facing changes. They should not be added for internal code changes.
