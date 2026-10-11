# OSS-Fuzz builder image for tpt-colour. Copy this directory to projects/tpt-colour in
# google/oss-fuzz to submit. The fuzz targets live in the repository's `fuzz/` workspace.
FROM gcr.io/oss-fuzz-base/base-builder-rust

RUN git clone --depth 1 https://github.com/tpt-solutions/tpt-colour.git tpt-colour

WORKDIR tpt-colour
COPY build.sh $SRC/
