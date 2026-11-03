#!/bin/sh
# Usage: sh run-facts.sh <folder holding the site copies>
# One process per site: veneer allows one open project per process. A site's failure is logged and
# does not stop the others.
here=$(dirname "$0")
for site in gitpress.app huttspawn rafters.studio runlegion.dev sean.silvius.me smugglr.dev; do
  node "$here/facts.ts" "$1/$site" "$here/results/facts" "$site" 2>&1 || echo "$site: FAILED, exit $?"
done
