#!/bin/sh
# Constitution V, FR-013, FR-014: the library uses only Relate's public library
# interface, and does not depend on the `relate` command-line tool.
set -eu
cd "$(dirname "$0")/.."

allowed="Connection Value Database QueryResult Error Result"
status=0

# Every item imported from `relate` in src/ must be on the allowed list.
used=$(grep -rhoE 'relate::(\{[^}]*\}|[A-Za-z_]+)' src | sed -E 's/relate:://; s/[{}]//g; s/,/ /g' | tr ' ' '\n' | sed '/^$/d' | sort -u)
for item in $used; do
  case " $allowed " in
    *" $item "*) ;;
    *) echo "src/ uses relate::$item, which is not on the allowed list ($allowed)"; status=1 ;;
  esac
done

# No dependency on the command-line crate or on engine internals.
if grep -nE '^relate-(cli|bin|core|storage|executor|planner|parser|front|ir|model)\b' Cargo.toml; then
  echo "Cargo.toml depends on a relate crate other than the public \`relate\` library"
  status=1
fi

[ "$status" -eq 0 ] && echo "public API check passed"
exit "$status"
