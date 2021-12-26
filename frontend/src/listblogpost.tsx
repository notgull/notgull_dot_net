// GNU AGPL v3 License

import React, { Component } from "react";

import BlogpostList from "./blogpostlist";
import Loading from "./loading";
import { Blogpost } from "./models";
import { list, ListParameters } from "./api";
import { Empty, LoadingState, PageSize } from "./util";

interface ListBlogpostState {
    // api-related loads
    loadstate: LoadingState,
    blogposts: Blogpost[] | undefined,

    // filters and pagination
    page_size: PageSize,
    page_index: number,
    title: string,
    tags: string,
}

export class ListBlogpost extends Component<Empty, ListBlogpostState> {
    state = {
        loadstate: LoadingState.Unmounted,
        blogposts: undefined,
        page_size: PageSize._25,
        page_index: 0,
        title: "",
        tags: "",
    };

    // update the blogposts variable based on updates to the state
    private updateBlogposts() {
        this.setState({
            loadstate: LoadingState.Loading,
            blogposts: undefined,
        });

        const params: ListParameters<Blogpost> = {
            page_size: this.state.page_size,
            page_index: this.state.page_index,
        };

        if (this.state.title.length > 0) {
            params.title = this.state.title;
        }
        if (this.state.tags.length > 0) {
            params.tags = this.state.tags;
        }

        list<Blogpost>("blogpost", params).then((blogposts) => {
            console.log(blogposts);
            this.setState({
                loadstate: LoadingState.Loaded,
                blogposts,
            });
        });
    }

    componentDidMount() {
        this.updateBlogposts();
    }

    render() {
        if (this.state.loadstate == LoadingState.Loading) {
            return <Loading />;
        } else if (this.state.loadstate == LoadingState.Loaded) {
            return <BlogpostList blogposts={this.state.blogposts!} />;
        } else {
            // component is mounting, eat the error
            return <></>;
        }
    }
};

export default ListBlogpost;