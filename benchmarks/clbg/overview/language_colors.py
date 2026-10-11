"""GitHub Linguist colors, with branding fallbacks, for white LOC charts.

GitHub's exact color takes precedence whenever its language registry defines
one, except for the requested Perl color override to distinguish it from Go.
Mech and LuaJIT are absent, so use their project branding. Dark outlines keep
bright markers readable without changing their fill or line color.
See language-color-sources.json for the fetched registry's digest and mapping.
"""

COLORS = dict(
    c='#555555',
    chapel='#8dc63f',
    cpp='#f34b7d',
    go='#00ADD8',
    java='#b07219',
    julia='#a270ba',
    mech='#F6C04E',         # Branding fallback: Mech wordmark gold.
    rust='#dea584',
    python='#3572A5',
    ruby='#701516',
    perl='#218C7F',         # Requested visual override: dark teal, distinct from Go.
    lua='#000080',
    luajit='#2142BF',       # Branding fallback: LuaJIT.org blue.
    javascript='#f1e05a',
    php='#4F5D95',
)

# Lines match the selected hue; Mech also uses its wordmark's shadow gold.
LINE_COLORS = dict(COLORS, mech='#996900')
OUTLINE_COLORS = dict(LINE_COLORS, javascript='#A58F00', go='#008AAE')
MECH_PROFILES = dict(fast=COLORS['mech'], small=LINE_COLORS['mech'])

# Palette source and the two primary project fallback assets:
# https://github.com/github-linguist/linguist/blob/main/lib/linguist/languages.yml
# https://mech-lang.org/img/logo.png (main #F6C04E; shadow #996900)
# https://luajit.org/bluequad.css (#2142BF)
