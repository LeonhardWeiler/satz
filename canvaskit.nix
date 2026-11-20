{ stdenv, lib, fetchgit, emscripten, gn, ninja, python3 }:

let
  externals = {
    freetype = fetchgit {
      url = "https://chromium.googlesource.com/chromium/src/third_party/freetype2.git";
      rev = "264b5fbf5b912b39f98d038bf75d39be0a73f21b";
      hash = "sha256-RRoTXV063rJBTL/JkVXuRZiNbPZLjtUP/kvt6XybG/k=";
    };
    libpng = fetchgit {
      url = "https://skia.googlesource.com/third_party/libpng.git";
      rev = "d5515b5b8be3901aac04e5bd8bd5c89f287bcd33";
      hash = "sha256-N507gFyjXXsJ3dyQN0SlVy80WXlbNlZs/cnqLaCmFTk=";
    };
    zlib = fetchgit {
      url = "https://chromium.googlesource.com/chromium/src/third_party/zlib";
      rev = "646b7f569718921d7d4b5b8e22572ff6c76f2596";
      hash = "sha256-jNj6SuTZ5/a7crtYhxW3Q/TlfRMNMfYIVxDlr7bYdzQ=";
    };
  };
in
stdenv.mkDerivation {
  pname = "canvaskit";
  version = "0.42.0";

  src = fetchgit {
    url = "https://skia.googlesource.com/skia";
    rev = "e2def78fa232884d4aedcf3ad92d4a0dd673b898";
    hash = "sha256-KwFJP2EqY3FLE1rUDkJl4X3OWCycw1PwVA4KUyj2w7o=";
  };

  nativeBuildInputs = [ emscripten gn ninja python3 ];

  postPatch = ''
    mkdir -p third_party/externals
    ${lib.concatStrings (lib.mapAttrsToList (n: p: "cp -r ${p} third_party/externals/${n}\n") externals)}
    chmod -R u+w third_party/externals
    printf '#!/bin/sh\n' > bin/activate-emsdk
    sed -i '/NODEJS_CATCH_EXIT/d' modules/canvaskit/BUILD.gn
    mkdir -p emsdk/upstream
    ln -s ${emscripten}/share/emscripten emsdk/upstream/emscripten
  '';

  configurePhase = ''
    export EM_CACHE=$TMPDIR/em-cache
    cp -r ${emscripten}/share/emscripten/cache $EM_CACHE
    chmod -R u+w $EM_CACHE
    gn gen out --args="
      is_debug=false is_official_build=true is_component_build=false is_trivial_abi=true
      is_canvaskit=true werror=false target_cpu=\"wasm\" skia_emsdk_dir=\"$PWD/emsdk\"
      skia_enable_ganesh=true skia_enable_graphite=false skia_use_webgl=true skia_use_webgpu=false
      skia_use_angle=false skia_use_dawn=false skia_use_vulkan=false skia_use_dng_sdk=false
      skia_use_expat=false skia_use_fontconfig=false skia_use_lua=false skia_use_piex=false
      skia_use_freetype=true skia_use_system_freetype2=false skia_use_freetype_woff2=false
      skia_use_freetype_zlib=false skia_use_system_libpng=false skia_use_system_zlib=false skia_use_zlib=true
      skia_enable_fontmgr_custom_directory=false skia_enable_fontmgr_custom_embedded=true
      skia_enable_fontmgr_custom_empty=true
      skia_use_icu=false skia_use_client_icu=false skia_use_bidi=false skia_use_libgrapheme=false
      skia_use_icu4x=false skia_use_harfbuzz=false
      skia_use_libjpeg_turbo_decode=false skia_use_libjpeg_turbo_encode=false skia_use_no_jpeg_encode=true
      skia_use_libpng_decode=false skia_use_libpng_encode=false skia_use_no_png_encode=true
      skia_use_libwebp_decode=false skia_use_libwebp_encode=false skia_use_no_webp_encode=true
      skia_use_wuffs=false skia_use_partition_alloc=false skia_enable_pdf=false skia_enable_skottie=false
      skia_enable_skshaper=false skia_enable_skparagraph=false
      skia_canvaskit_enable_paragraph=false skia_canvaskit_enable_bidi=false
      skia_canvaskit_enable_font=true skia_canvaskit_enable_embedded_font=false
      skia_canvaskit_enable_alias_font=false skia_canvaskit_enable_skp_serialization=false
      skia_canvaskit_enable_effects_deserialization=false skia_canvaskit_enable_sksl_trace=false
      skia_canvaskit_enable_canvas_bindings=false skia_canvaskit_enable_matrix_helper=true
      skia_canvaskit_enable_pathops=true skia_canvaskit_enable_rt_shader=true
      skia_canvaskit_enable_webgl=true skia_canvaskit_enable_webgpu=false
      skia_canvaskit_force_tracing=false skia_canvaskit_profile_build=false
    "
  '';

  buildPhase = "ninja -C out canvaskit.js";

  installPhase = ''
    mkdir -p $out
    cp out/canvaskit.js out/canvaskit.wasm LICENSE $out
    sed -i 's/if(typeof exports==="object".*$/export default CanvasKitInit;/' $out/canvaskit.js
  '';
}
