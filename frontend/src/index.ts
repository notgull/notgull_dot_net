// GNU AGPL v3 License

import { h, render } from "preact";

import BlogpostCreate from "./blogcreate";
import BlogpostEdit from "./blogedit";
import DeletePost from "./deletepost";
import Frontpage from "./frontpage";
import getConsts from "./consts";
import ListBlogpost from "./listblogpost";
import navlink from "./navlink";
import UsernameForm from "./usernameform";
import UserInfo from "./user_info";
import UserList from "./user_list";

// If we find an element with this ID, load this component into it
const router: Record<string, any> = {
    "blogpost-list": ListBlogpost,
    "blogpost-edit": BlogpostEdit,
    "blogpost-create": BlogpostCreate,
    "blogpost-delete": DeletePost,
    "frontpage": Frontpage,
    "username-form": UsernameForm,
    "user-info": UserInfo,
    "user-list": UserList,
};

function navlinks() {
    const consts = getConsts();
    navlink(
        "login",
        "login",
        "Log In",
        () => consts.user_id === undefined,
    );
    navlink(
        "create_blogpost",
        "blog/create",
        "Create New Blogpost",
        () => (consts.user_perms & 0x1) != 0,
    );

    const blogpostId = consts.cur_blogpost_id;
    if (blogpostId !== undefined) {
        navlink(
            "edit_blogpost",
            `blog/edit/${blogpostId}`,
            "Edit",
            () => (consts.user_perms & 0x1) != 0,
        );

        navlink(
            "delete_blogpost",
            `blog/delete/${blogpostId}`,
            "Delete",
            () => (consts.user_perms & 0x1) != 0,
        );
    }
}

function main() {
    navlinks();

    for (const root_id in router) {
        // try to get an element with that ID
        const root_elem = document.getElementById(root_id);

        if (root_elem !== null) {
            console.log(`Found element "${root_id}"`);
            render(h(router[root_id], null, []), root_elem);
            break;
        }
    }
}

window.onload = () => main();