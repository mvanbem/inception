#!/bin/bash
set -e

function pushd {
    command pushd "$@" > /dev/null
}

function popd {
    command popd "$@" > /dev/null
}

function import {
    echo "Importing file: $1"
    cp ../bsp-loader-gl/"$1" bsp-loader-gx/assets/
}

function import_texture {
    echo "Converting texture: $1.png in format $2 ($3)"

    pushd ../bsp-loader-gl
    gxtexconv -i "$1".png colfmt="$2"
    mv "$1"{,_"$3"}.tpl
    rm "$1".h
    popd

    import "$1"_"$3".tpl
    rm ../bsp-loader-gl/"$1"_"$3".tpl
}

import position_data.dat
import texcoord_data.dat
import display_lists.dat
import bsp_nodes.dat
import bsp_leaves.dat
import vis.dat

import_texture lightmap_atlas 14 cmpr
import_texture lightmap_atlas 6 rgba8
