{
    "targets": [
        {
            "target_name": "turn",
            "sources": [
                "src/turn.cpp"
            ],
            "include_dirs": [
                "<!@(node -p \"require('node-addon-api').include\")",
                "../turn-exports"
            ],
            "libraries": [
                "../target/debug/libturn.lib"
            ],
            "cflags_cc": [
                "-std=c++20"
            ]
        }
    ]
}