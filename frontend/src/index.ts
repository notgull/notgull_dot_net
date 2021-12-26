// GNU AGPL v3 License

import React from "react";
import { render } from "react-dom";

import ListBlogpost from "./listblogpost";

// If we find an element with this ID, load this component into it
const router: Record<string, any> = {
    "blogpost-list": ListBlogpost,
};

function main() {
    for (const root_id in router) {
        // try to get an element with that ID
        const root_elem = document.getElementById(root_id);

        if (root_elem !== null) {
            console.log(`Found element "${root_id}"`);
            render(React.createElement(router[root_id], null, []), root_elem);
            return;
        }
    }
}

window.onload = () => main();