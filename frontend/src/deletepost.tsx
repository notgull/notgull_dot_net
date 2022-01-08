// GNU AGPL v3 License

import { Component, Fragment, h } from "preact";

import getConsts from "./consts";
import { doDelete } from "./api";
import { Empty } from "./util";

interface DeletePostState {
    uploading: boolean,
    error: string,
};

export class DeletePost extends Component<Empty, DeletePostState> {
    state = {
        uploading: false,
        error: "",
    };

    private deletePost() {
        if (!this.state.uploading) {
            this.setState({
                uploading: true
            });

            doDelete("blogpost", getConsts().cur_blogpost_id!).then(() => {
                const newUrl = `${getConsts().web_url}/blog`;
                window.location.href = newUrl;
            }).catch((err) => {
                this.setState({
                    uploading: false,
                    error: err.error,
                });
            });
        }
    }

    private cancelDeletion() {
        if (!this.state.uploading) {
            const newUrl = `${getConsts().web_url}/blog`;
            window.location.href = newUrl;
        }
    }

    render() {
        let errorElem = <></>;
        if (this.state.error.length > 0) {
            errorElem = <p>{this.state.error}</p>;
        }

        return (
            <>
                {errorElem}
                <button onClick={() => this.deletePost()}>Yes</button>
                <button onClick={() => this.cancelDeletion()}>No</button>
            </>
        );
    }
};

export default DeletePost;