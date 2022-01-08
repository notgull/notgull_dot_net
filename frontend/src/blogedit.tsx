// GNU AGPL v3 License

import { Component, Fragment, h } from "preact";

import BlogpostEditor, { UploadedBlogpost } from "./blogpost_editor";
import getConsts from "./consts";
import Loading from "./loading";
import { Blogpost } from "./models";
import { LoadingState, Empty } from "./util";
import { get, patch } from "./api";

interface BlogpostEditState {
    loadstate: LoadingState,
    error: string,
    blogpost: Blogpost | undefined,
};

export class BlogpostEdit extends Component<Empty, BlogpostEditState> {
    state = {
        loadstate: LoadingState.Unmounted,
        error: "",
        blogpost: undefined,
    }; 

    componentDidMount() {
        this.setState({
            loadstate: LoadingState.Loading,
        });

        const id = getConsts().user_id!;
        get<Blogpost>("blogpost", id).then((bp: Blogpost) => {
            this.setState({
                blogpost: bp,
                loadstate: LoadingState.Loaded,
            });
        });
    }

    uploadBlogpost(bp: UploadedBlogpost): Promise<Empty> {
        const consts = getConsts();
        return patch<Blogpost>("blogpost", consts.user_id!, bp).then(() => {
            const url = `${consts.web_url}/blog/${bp.url}`;
            window.location.href = url;

            // unreachable
            return {};
        });
    }

    render() {
        const { blogpost, error, loadstate } = this.state;
        const bp: Blogpost = blogpost!;

        if (loadstate == LoadingState.Unmounted) {
            return <></>;
        } else if (loadstate == LoadingState.Loading) {
            return <Loading />;
        } else if (loadstate == LoadingState.Loaded) {
            return (
                <BlogpostEditor
                    original_title={bp.title}
                    original_url={bp.url}
                    original_tags={bp.tags}
                    original_body={bp.body}
                    uploader={(bp) => this.uploadBlogpost(bp)} />
            );
        } else {
            return <p>{error}</p>;
        }
    }
};

export default BlogpostEdit;