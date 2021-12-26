// GNU AGPL v3 License

import React, { Component } from "react";

import BlogpostListItem from "./listitem";
import { Blogpost } from "../models";
import { Empty } from "../util";

interface BlogpostListProps {
    blogposts: Blogpost[],
}

export class BlogpostList extends Component<BlogpostListProps, Empty> {
    render() {
        return this.props.blogposts.map((blogpost, i) => (
            <BlogpostListItem key={i} blogpost={blogpost} />
        ));
    }
};

export default BlogpostList;