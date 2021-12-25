// GNU AGPL v3 License

"use strict";

const { src, dest, series, parallel, task } = require("gulp");

const webpack = require("webpack-stream");
const webpack_config = require("./webpack.config");

const fs = require("fs");
const path = require("path");

// Settings
const MODE = "development";

// Constants
const NODE_MODULES = "./node_modules/";
const PUBLIC_DIR = "./public/";
const SOURCE_DIR = "./src/";

// set webpack_config development specifier
webpack_config.mode = MODE;

// Create a folder, recursively.
function createFolder(path) {
    return (cb) => {
        fs.mkdir(path, { recursive: true }, cb);
    };
}

// Copy a file from ./node_modules/{package}/{relUri} to ./public/{dir}/{package}.{desiredExt}
function copyNodeFile(pack, relUri, dir = "js", desiredExt = "js") {
    return (cb) => {
        const infile = path.join(NODE_MODULES, pack, relUri);
        const outfile = path.join(PUBLIC_DIR, dir, pack + "." + desiredExt);

        fs.copyFile(infile, outfile, cb);
    };
}

// Run webpack and output the files to the public dir.
function runWebpack() {
    return src(path.join(SOURCE_DIR, "index.ts"))
        .pipe(webpack(webpack_config, null, (err, stats) => {
            // Do nothing with these
        }))
        .pipe(dest(path.join(PUBLIC_DIR, "js")));
}

// Create necessary directories
const createPublicDir = createFolder(PUBLIC_DIR);
const createJsDir = createFolder(path.join(PUBLIC_DIR, "js"));
const createDirs = parallel(createPublicDir, createJsDir);

// Copy files from node_modules to public/js
const REACT_PATH = MODE == "development" ? "react.development.js" : "react.production.min.js";
const RD_PATH = MODE == "development" ? "react-dom.development.js" : "react-dom.production.min.js";
const MODULES_TO_COPY = [
    ["axios", "dist/axios.min.js"],
    ["react", path.join("umd", REACT_PATH)],
    ["react-dom", path.join("umd", RD_PATH)],
];

const copyModulesTasks = MODULES_TO_COPY.map((module) => {
    const [pack, relUri] = module;
    return copyNodeFile(pack, relUri);
});
const copyModules = parallel(...copyModulesTasks);

// Run webpack
const doesRunWebpack = runWebpack;

// Build the frontend.
const buildFrontend = series(createDirs, parallel(copyModules, doesRunWebpack));

// Default task is to build the frontend.
exports.default = buildFrontend;