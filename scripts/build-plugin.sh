#!/usr/bin/env bash
# Builds the Docker image of a plugin and exports it as the .tar.gz that HCUweb installs.
#
#   scripts/build-plugin.sh <plugin> [--version X.Y.Z] [--out DIR] [--no-save]
#   scripts/build-plugin.sh --all [...]
#   scripts/build-plugin.sh --list
#
# <plugin> is a directory name below plugins/. Image repository is the plugin id from
# [package.metadata.hcu] in the plugin's Cargo.toml, the tag is the version (default: the
# package version). Output: <out>/<plugin-id>-<version>.tar.gz (default out: dist/).
# Requires: docker (buildx), cargo, jq.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

version="" out="dist" save=1 plugins=()

usage() { sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'; }
die() { echo "error: $*" >&2; exit 1; }
list_plugins() { for d in plugins/*/; do [ -f "$d/Cargo.toml" ] && basename "$d"; done; }

while [ $# -gt 0 ]; do
  case "$1" in
    --version) version="${2:?--version needs a value}"; shift 2 ;;
    --out) out="${2:?--out needs a value}"; shift 2 ;;
    --no-save) save=0; shift ;;
    --all) mapfile -t plugins < <(list_plugins); shift ;;
    --list) list_plugins; exit 0 ;;
    -h|--help) usage; exit 0 ;;
    -*) usage >&2; die "unknown option $1" ;;
    *) plugins+=("$1"); shift ;;
  esac
done
[ ${#plugins[@]} -gt 0 ] || { usage >&2; exit 1; }
[ -z "$version" ] || version="${version#v}"

for tool in docker cargo jq; do
  command -v "$tool" >/dev/null || die "$tool is required"
done
docker buildx version >/dev/null 2>&1 || die "docker buildx is required"

metadata="$(cargo metadata --no-deps --format-version 1 --locked)"

build_plugin() {
  local plugin="$1" dir="plugins/$1"
  [ -f "$dir/Cargo.toml" ] || die "no such plugin '$plugin' (see --list)"

  # Package + [package.metadata.hcu] of this plugin.
  local pkg
  pkg="$(jq -ce --arg dir "$root/$dir/Cargo.toml" '.packages[] | select(.manifest_path == $dir)' <<<"$metadata")" \
    || die "$dir is not a workspace member"
  jq -e '.metadata.hcu' <<<"$pkg" >/dev/null || die "$dir/Cargo.toml has no [package.metadata.hcu]"
  local key
  for key in plugin-id issuer hcu-min-version scope friendly-name; do
    jq -e --arg k "$key" '.metadata.hcu[$k]' <<<"$pkg" >/dev/null \
      || die "$dir/Cargo.toml: package.metadata.hcu.$key is required"
  done
  jq -e '.metadata.hcu["friendly-name"].de' <<<"$pkg" >/dev/null \
    || die "$dir/Cargo.toml: package.metadata.hcu.friendly-name needs a 'de' entry"

  local plugin_id v
  plugin_id="$(jq -r '.metadata.hcu["plugin-id"]' <<<"$pkg")"
  v="${version:-$(jq -r '.version' <<<"$pkg")}"

  # Optional files referenced from the metadata, relative to the plugin directory.
  local image="" changelog="" file
  file="$(jq -r '.metadata.hcu.image // empty' <<<"$pkg")"
  [ -z "$file" ] || { [ -f "$dir/$file" ] || die "image file $dir/$file not found"; image="$(base64 -w0 "$dir/$file")"; }
  file="$(jq -r '.metadata.hcu.changelog // empty' <<<"$pkg")"
  [ -z "$file" ] || { [ -f "$dir/$file" ] || die "changelog file $dir/$file not found"; changelog="$(cat "$dir/$file")"; }

  # JSON for the de.eq3.hmip.plugin.metadata label (Connect API documentation, 4.1).
  local label
  label="$(jq -c --arg v "$v" --arg image "$image" --arg changelog "$changelog" '
    .metadata.hcu as $m | {
      pluginId: $m["plugin-id"],
      issuer: $m.issuer,
      version: $v,
      hcuMinVersion: $m["hcu-min-version"],
      scope: $m.scope,
      friendlyName: $m["friendly-name"],
      description: $m.description,
      settings: $m.settings,
      image: (if $image == "" then null else $image end),
      changelog: (if $changelog == "" then null else $changelog end),
      logsEnabled: ($m["logs-enabled"] // false)
    } | with_entries(select(.value != null))' <<<"$pkg")"

  local tag="$plugin_id:$v"
  echo "==> building $tag (linux/arm64)"
  docker buildx build \
    --platform linux/arm64 \
    --build-arg "PLUGIN=$plugin" \
    --build-arg "VERSION=$v" \
    --build-arg "HCU_METADATA=$label" \
    --tag "$tag" \
    --load \
    --file docker/plugin.Dockerfile \
    .

  local size
  size="$(docker image inspect "$tag" --format '{{.Size}}')"
  echo "    image $tag: $((size / 1024)) KiB"

  if [ "$save" -eq 1 ]; then
    mkdir -p "$out"
    local archive="$out/$plugin_id-$v.tar.gz" compress=(gzip -9)
    command -v pigz >/dev/null && compress=(pigz -9)
    docker save "$tag" | "${compress[@]}" >"$archive"
    echo "    archive $archive ($(du -h "$archive" | cut -f1), sha256 $(sha256sum "$archive" | cut -d' ' -f1))"
  fi
}

for plugin in "${plugins[@]}"; do
  build_plugin "$plugin"
done
