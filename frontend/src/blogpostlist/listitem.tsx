// GNU AGPL v3 License

import { h } from "preact";

import { Blogpost } from "../models";

interface BlogpostListItemProps {
    blogpost: Blogpost,
}

export function BlogpostListItem(props: BlogpostListItemProps) {
    const blogpost = props.blogpost;
    const fullUrl = `/blog/${blogpost.url}`;
    
    return (
        <div className="blogpost-list-item">
            <h2><a href={fullUrl}>{blogpost.title}</a></h2>
            <div className="blogpost-list-item-body">
                {blogpost.body}
            </div>
        </div>
    );
};

export default BlogpostListItem;