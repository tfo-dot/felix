#!/usr/bin/env nu

let tile_w = 256
let tile_h = 256
let cols = 8
let out_png = "assets/assets_spritesheet.png"
let out_json = "assets/assets_spritesheet.json"

let meta_png = "assets/pet_spritesheet_meta.png"
let colors_json = "assets/pet_spritesheet_meta_colors.json"

def main [] {
    let svg_files = (try { ls assets/svg/*.svg | get name } catch { [] })
    
    if ($svg_files | is-empty) {
        print "No SVG files found."
        return
    }

    print "Converting SVGs to PNGs..."
    
    let png_files = ($svg_files | each {|f|
        let png_name = $"($f).png"
        
        print $"  -> Exporting ($png_name)"
        
        ^inkscape $f --export-type=png --export-filename=($png_name) -w $tile_w -h $tile_h
        
        $png_name
    })

    print "Stitching spritesheet..."
    ^magick montage ...$png_files -tile $"($cols)x" -geometry +0+0 -background none $out_png

    print "Generating JSON map..."
    
    let frames = ($png_files | enumerate | reduce --fold [] {|it, acc|
        let col = ($it.index mod $cols)
        let row = ($it.index // $cols)
        
        let x = ($col * $tile_w)
        let y = ($row * $tile_h)
        let name = ($it.item | path parse | get stem | path parse | get stem)
        
        $acc | append { x: $x, y: $y, name: $name }
    })

    $frames | to json | save --force $out_json

    print $"Success! Created ($out_png) and ($out_json)."

    ./extract_rig_markers.nu assets/pet_spritesheet_meta.png assets/pet_spritesheet_meta_colors.json 256 256 --out assets/pet_meta.json
}