#!/usr/bin/env bash
set -euo pipefail

cargo build

agd_bin="${CARGO_TARGET_DIR:-target}/debug/agd"
if [[ ! -x "$agd_bin" && -x ".devenv/state/target/debug/agd" ]]; then
  agd_bin=".devenv/state/target/debug/agd"
fi
if [[ ! -x "$agd_bin" ]]; then
  echo "Could not find built agd binary" >&2
  exit 1
fi
agd_bin="$(cd "$(dirname "$agd_bin")" && pwd -P)/$(basename "$agd_bin")"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

human="$tmp/human"
agd_home="$tmp/agd-home"
fake_gpg="$tmp/fake-gpg"

mkdir -p "$human" "$agd_home"
cat >"$fake_gpg" <<'SCRIPT'
#!/usr/bin/env bash
cat >/dev/null
echo '[GNUPG:] SIG_CREATED D 1 10 00 0 0 0 0' >&2
printf '%s\n' '-----BEGIN PGP SIGNATURE-----' '' 'fake' '-----END PGP SIGNATURE-----'
SCRIPT
chmod +x "$fake_gpg"

git -C "$human" init -b main
git -C "$human" config user.name "Human Developer"
git -C "$human" config user.email "human@example.test"
git -C "$human" config gpg.format openpgp
git -C "$human" config user.signingkey human@example.test
git -C "$human" config gpg.program "$fake_gpg"
printf '# proof\n' >"$human/README.md"
git -C "$human" add README.md
git -C "$human" commit -m "initial"

cd "$human"
AGD_HOME="$agd_home" "$agd_bin" init
workspace="$(cd "$human" && AGD_HOME="$agd_home" "$agd_bin" path)"

git -C "$workspace" switch -c agent/manual-proof
printf 'one\n' >"$workspace/agent.txt"
git -C "$workspace" add agent.txt
git -C "$workspace" commit -m "agent one"
printf 'two\n' >"$workspace/agent.txt"
git -C "$workspace" add agent.txt
git -C "$workspace" commit -m "agent two"

author="$(git -C "$workspace" log -1 --format='%an <%ae>')"
test "$author" = "Local Agent <agent@agd.invalid>"

printf 'signed\n' >"$workspace/signed.txt"
git -C "$workspace" add signed.txt
if git -C "$workspace" commit -S -m "signed work" 2>"$tmp/signing.err"; then
  echo "Expected explicit signing to fail" >&2
  exit 1
fi
grep -q "AGD denied explicit signing" "$tmp/signing.err"
test -f "$agd_home/logs/signing-denials.jsonl"
git -C "$workspace" reset --hard HEAD

pushurl="$(git -C "$workspace" remote get-url --push origin)"
test "$pushurl" = "agd-deny://push-disabled"

cd "$human"
AGD_HOME="$agd_home" "$agd_bin" bless agent/manual-proof

git -C "$human" log -1 --show-signature --format=full
git -C "$human" log -1 --format=%B | grep -q "AGD-Agent-Branch: agent/manual-proof"

echo "manual proof passed"
