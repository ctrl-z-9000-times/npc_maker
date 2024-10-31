#!/usr/bin/bash -x

#
# Go to the root of the repository.
#

REPO=`git rev-parse --show-toplevel`
cd $REPO

# 
# Run pandoc to convert from markdown to html.
# 

pandoc -f gfm -s -o docs/The_NPC_Maker.html \
    --metadata title=The\ NPC\ Maker \
    --metadata content="width=device-width, initial-scale=1.0" \
    --shift-heading-level-by=1 \
    --css=print.css \
    README.md \
    docs/environments.md \
    docs/controllers.md \
    docs/evolution.md \
    docs/management.md \
    environments.md \
    controllers.md \

#
# Open the HTML documentation.
#

python -m http.server &
firefox 0.0.0.0:8000/docs/The_NPC_Maker.html

read -p "~~~~~~~~~~~~~~~~~ PRESS ENTER TO QUIT ~~~~~~~~~~~~~~~~~      " </dev/tty
pkill -f "http.server"

