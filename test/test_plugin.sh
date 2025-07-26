#!/bin/bash

pluginval.app/Contents/MacOS/pluginval --validate-in-process --output-dir "./bin" \
    "../treble/release/bundled/ahmad.vst3/Contents/MacOS/ahmad" || exit 1
