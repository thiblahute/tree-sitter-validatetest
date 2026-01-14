{
  "targets": [
    {
      "target_name": "tree_sitter_validatetest_binding",
      "dependencies": [
        "<!(node -p \"require('node-addon-api').targets\"):node_addon_api_except",
      ],
      "include_dirs": [
        "src",
      ],
      "sources": [
        "bindings/node/binding.cc",
        "src/parser.c",
      ],
      "conditions": [
        ["OS!='win'", {
          "cflags_c": ["-std=c11"],
          "cflags_cc": ["-std=c++17"]
        }]
      ],
      "xcode_settings": {
        "CLANG_CXX_LANGUAGE_STANDARD": "c++17",
        "GCC_C_LANGUAGE_STANDARD": "c11"
      }
    }
  ]
}
