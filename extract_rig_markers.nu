#!/usr/bin/env nu
# extract_rig_markers.nu
#
# Combines color-coded base markers with their (optional) angle-indicator
# markers, grouped per JSON color key, e.g.:
#
#   { "eyes": "#FFFFFF", "head": "#FAFAFA", ... }
#
# Each key's color identifies its base marker(s) (painted at full opacity)
# and — if a rotation is needed — a same-colored angle marker painted at
# partial opacity nearby. Pairing is done by grid tile (frame_width x
# frame_height) + color, not just nearest-neighbor: since markers are
# unique per tile, "which base does this angle mark belong to" is really
# "which base of the same color shares this tile", which is exact rather
# than distance-based guessing.
#
# Output shape:
#   { "eyes": [ {"x":.., "y":.., "tile_col":.., "tile_row":.., "tile_index":.., "rotation":..}, ... ], "head": [...], ... }
# x/y are the marker's position *within its grid tile* (0..frame_width-1,
# 0..frame_height-1) rather than absolute image coordinates — e.g. a base
# marker at absolute (605, 221) in a 256x256 grid is reported as (93, 221).
# tile_col/tile_row are the 0-based grid column/row the marker's tile sits
# in; tile_index = tile_row * columns + tile_col, using the image's actual
# width to compute columns — so it stays correct even when some tiles have
# no markers at all (you can't safely reconstruct that from array position
# alone once entries are missing).
# rotation is in radians, measured from the base marker to the angle
# marker (0 for a base with no paired angle marker).
#
# Requires: ImageMagick's `convert` on PATH (v6) — for IMv7 swap `convert`
# for `magick`.
#
# Usage:
#   nu extract_rig_markers.nu meta.png colors.json 256 256
#   nu extract_rig_markers.nu meta.png colors.json 256 256 --flip-y --out rig.json

def atan2 [y: float, x: float] {
    if $x > 0 {
        ($y / $x) | math arctan
    } else if $x < 0 and $y >= 0 {
        (($y / $x) | math arctan) + 3.141592653589793
    } else if $x < 0 and $y < 0 {
        (($y / $x) | math arctan) - 3.141592653589793
    } else if $x == 0 and $y > 0 {
        1.5707963267948966
    } else if $x == 0 and $y < 0 {
        -1.5707963267948966
    } else {
        0.0   # base and angle point coincide — undefined direction
    }
}

def hex-to-rgb [hex: string] {
    let clean = ($hex | str trim | str trim -c '#' | str uppercase)
    {
        r: ($clean | str substring 0..1 | into int -r 16)
        g: ($clean | str substring 2..3 | into int -r 16)
        b: ($clean | str substring 4..5 | into int -r 16)
    }
}

def main [
    image: path            # path to the meta map PNG
    colors: path            # path to JSON color map: { "key": "#RRGGBB", ... }
    frame_width: int        # sprite frame/tile width in px (grid cell size)
    frame_height: int       # sprite frame/tile height in px
    --flip-y                # report rotation with +y pointing up instead of raw pixel-space down
    --out: path              # optional: save result here (.json / .nuon by extension)
] {
    if not ($image | path exists) {
        error make {msg: $"Image not found: ($image)"}
    }
    if not ($colors | path exists) {
        error make {msg: $"Colors file not found: ($colors)"}
    }

    let dims = (^identify -format "%w %h" $image | complete)
    if $dims.exit_code != 0 {
        error make {msg: $"identify failed: ($dims.stderr)"}
    }
    let wh = ($dims.stdout | str trim | split row " ")
    let img_width = ($wh | get 0 | into int)
    let img_height = ($wh | get 1 | into int)
    let cols = ($img_width // $frame_width)

    let color_map = (open $colors)
    # key -> {r,g,b}, preserving the original key order for the final output
    let keys_ordered = ($color_map | columns)
    let color_lookup = (
        $color_map
        | transpose key hex
        | each {|row| {key: $row.key} | merge (hex-to-rgb $row.hex)}
    )

    # No thresholding: we need the real color + alpha of every blob so we
    # can both identify its key (by color) and its role, base vs. angle
    # marker (by opacity).
    let raw = (
        ^convert $image
            -define connected-components:verbose=true
            -connected-components 8
            null:
        | complete
    )
    if $raw.exit_code != 0 {
        error make {msg: $"ImageMagick failed: ($raw.stderr)"}
    }

    let blobs = (
        $raw.stdout
        | lines
        | skip 1
        | parse -r '^\s*(?P<id>\d+):\s+(?P<w>\d+)x(?P<h>\d+)\+(?P<x>\d+)\+(?P<y>\d+)\s+(?P<cx>[\d.]+),(?P<cy>[\d.]+)\s+(?P<area>\d+)\s+srgba\((?P<r>\d+),(?P<g>\d+),(?P<b>\d+),(?P<alpha>[\d.]+)\)'
        | each {|row| {
            center_x: ($row.cx | into float)
            center_y: ($row.cy | into float)
            r: ($row.r | into int)
            g: ($row.g | into int)
            b: ($row.b | into int)
            alpha: ($row.alpha | into float)
        }}
        | where alpha > 0    # drop the single untouched-background blob
        | each {|row|
            let matches = ($color_lookup | where r == $row.r and g == $row.g and b == $row.b)
            let key = if ($matches | is-empty) { null } else { ($matches | first).key }
            $row | insert key $key | insert frame_col ($row.center_x // $frame_width | into int) | insert frame_row ($row.center_y // $frame_height | into int)
        }
    )

    let unknown = ($blobs | where key == null)
    if ($unknown | length) > 0 {
        print $"Warning: ($unknown | length) marker\(s) had a color that isn't in ($colors) and were ignored."
    }

    let known = ($blobs | where key != null)

    mut result = {}
    for k in $keys_ordered {
        let group = ($known | where key == $k)
        if ($group | is-empty) {
            $result = ($result | insert $k [])
            continue
        }
        # per color: highest alpha present = base marks, everything else = angle marks
        let base_alpha = ($group | get alpha | math max)
        let bases = ($group | where alpha == $base_alpha)
        let angles = ($group | where alpha != $base_alpha)

        mut entries = []
        for base in $bases {
            let same_tile_candidates = (
                $angles
                | where frame_col == $base.frame_col and frame_row == $base.frame_row
            )
            let same_tile_angle = if ($same_tile_candidates | is-empty) { null } else { $same_tile_candidates | first }
            let rotation = if ($same_tile_angle == null) {
                0.0
            } else {
                let dx = ($same_tile_angle.center_x - $base.center_x)
                mut dy = ($same_tile_angle.center_y - $base.center_y)
                if $flip_y {
                    $dy = (-1.0) * $dy
                }
                (atan2 $dy $dx)
            }
            $entries = ($entries | append {
                x: (($base.center_x mod $frame_width) | into int)
                y: (($base.center_y mod $frame_height) | into int)
                tile_col: $base.frame_col
                tile_row: $base.frame_row
                tile_index: ($base.frame_row * $cols + $base.frame_col)
                rotation: ($rotation | math round --precision 6)
            })
        }

        # flag angle marks that never found a same-tile base — silent
        # otherwise, since they'd just be dropped.
        let orphan_angles = ($angles | where {|a| not ($bases | any {|b| $b.frame_col == $a.frame_col and $b.frame_row == $a.frame_row}) })
        if ($orphan_angles | length) > 0 {
            print $"Warning: ($orphan_angles | length) angle marker\(s) for '($k)' had no base marker in the same tile."
        }

        $result = ($result | insert $k ($entries | sort-by tile_index))
    }

    if ($out | is-not-empty) {
        let ext = ($out | path parse | get extension)
        match $ext {
            "nuon" => ($result | to nuon | save -f $out)
            _      => ($result | to json --indent 2 | save -f $out)
        }
        print $"Wrote ($out)"
    }

    $result
}
