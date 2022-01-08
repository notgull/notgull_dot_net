// GNU AGPL v3 License

import { Component, Fragment, h } from "preact";

import BlogpostList from "./blogpostlist";
import DebouncedSearch from "./debounced_search";
import Loading from "./loading";
import Pagination from "./pagination";
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

    error?: string,
}

export class ListBlogpost extends Component<Empty, ListBlogpostState> {
    state = {
        loadstate: LoadingState.Unmounted,
        blogposts: undefined,
        page_size: PageSize._25,
        page_index: 0,
        title: "",
        tags: "",
        error: "",
    };

    private updateSearch(title: string) {
        this.setState({ title });
        this.updateBlogposts();
    }

    // update the blogposts variable based on updates to the state
    private updateBlogposts() {
        this.setState({
            loadstate: LoadingState.Loading,
            blogposts: undefined,
        });

        const params: ListParameters<Blogpost> = {
            skip: this.state.page_size * this.state.page_index,
            count: this.state.page_size,
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
        }).catch((err) => {
            this.setState({
                loadstate: LoadingState.ErrorOccurred,
                error: err.error,
            })
        })
    }

    componentDidMount() {
        this.updateBlogposts();
    }

    render() {
        let bodyComponent = <></>;
        if (this.state.loadstate == LoadingState.Loading) {
            bodyComponent = <Loading />;
        } else if (this.state.loadstate == LoadingState.Loaded) {
            bodyComponent = (
                <>
                    <BlogpostList blogposts={this.state.blogposts!} />
                </>
            );
        } else if (this.state.loadstate == LoadingState.ErrorOccurred) {
            bodyComponent = (
                <p>
                    Unable to load blogposts: {this.state.error!}
                </p>
            )
        } else {
            // component is mounting, eat the error
            bodyComponent = <></>;
        }

        const setPage = (page: number) => {
            this.setState({
                page_index: page,
            });

            this.updateBlogposts();
        };

        return (
            <>
                <DebouncedSearch 
                 debounceTime={1000}
                 onChange={(s) => this.updateSearch(s)}
                 startingSearch={this.state.title} /> 
                {bodyComponent}
                <Pagination
                 page={this.state.page_index}
                 setPage={setPage} />
            </>
        )
    }
};

export default ListBlogpost;