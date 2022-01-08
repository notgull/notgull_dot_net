// GNU AGPL v3 License

import { Component, h } from "preact";

import BlogpostEditor, { UploadedBlogpost } from "./blogpost_editor";
import { Empty } from "./util";
import { post } from "./api";

export function BlogpostCreate(props: Empty) {
    const uploader = (ub: UploadedBlogpost) => (
        post("blogpost", ub).then((_) => ({}))
    );

    return (
        <BlogpostEditor original_title=""
                        original_tags=""
                        original_body=""
                        original_url=""
                        uploader={uploader} />
    );
};

export default BlogpostCreate;