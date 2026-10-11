# OSS-Fuzz staging

This directory holds the project files for [OSS-Fuzz](https://github.com/google/oss-fuzz). They are
not submitted from here. Submitting means opening a pull request against google/oss-fuzz from the
maintainer's account, which is a decision for the maintainer.

## Contents

- `projects/tpt-colour/project.yaml`: project metadata. Replace `primary_contact` before submitting.
- `projects/tpt-colour/Dockerfile`: builder image. It clones this repository.
- `projects/tpt-colour/build.sh`: builds the four targets in `fuzz/` and packages the seed corpora.

## Submitting

1. Fork google/oss-fuzz.
2. Copy `projects/tpt-colour` into `projects/tpt-colour` of the fork.
3. Set `primary_contact` in `project.yaml`.
4. Check the build locally, if Docker is available:
   `python infra/helper.py build_image tpt-colour` then `python infra/helper.py build_fuzzers tpt-colour`
   and `python infra/helper.py check_build tpt-colour`.
5. Open the pull request.

## Status

Checked locally in the OSS-Fuzz Rust builder image (`gcr.io/oss-fuzz-base/base-builder-rust`).
`build.sh` ran to completion against this repository, and built the four targets with sanitizers.
Each binary was then run for 20 seconds with its seed corpus, with no crashes. `parse_css` ran
about 4.3 million inputs.

Not done: the submission itself. The fork, the pull request and the contact address are the
maintainer's to set.
