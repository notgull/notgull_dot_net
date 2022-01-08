// GNU AGPL v3 License

import { Component, Fragment, h } from "preact";

import ImageUploader from "./image_uploader";
import { capitalize, Empty } from "./util";
import { getConsts } from "./consts";

export interface UploadedBlogpost {
    title: string,
    tags: string,
    url: string,
    body: string,
    author_id: number,
};

interface BlogpostEditorProps {
    original_title: string,
    original_url: string,
    original_tags: string,
    original_body: string,
    uploader: (ub: UploadedBlogpost) => Promise<Empty>,
};

interface BlogpostEditorState {
    ub: UploadedBlogpost,
    uploading: boolean,
    error?: string,
}

export class BlogpostEditor extends Component<BlogpostEditorProps, BlogpostEditorState> {
    state = {
        ub: { title: "", tags: "", url: "", body: "", author_id: getConsts().user_id! },
        uploading: false,
        error: undefined,
    };

    componentDidMount() {
        this.setState({
            ub: {
                title: this.props.original_title,
                tags: this.props.original_tags,
                url: this.props.original_url,
                body: this.props.original_body,
                author_id: getConsts().user_id!,
            }
        });
    }

    doUpload() {
        this.setState({ uploading: true });
        this.props.uploader(this.state.ub).then((_) => {
            const consts = getConsts();
            window.location.href = `${consts.web_url}/blog/${this.state.ub.url}`;
        }).catch((err) => {
            this.setState({
                uploading: false,
                error: err.error,
            });
        })
    }

    render() {
        const { title, tags, url, body } = this.state.ub;
        const ubChanger = (field: string) => (value: string) => this.setState({
            ub: Object.assign(this.state.ub, { [field]: value }),
        });
        const setTitle = ubChanger("title");
        const setTags = ubChanger("tags");
        const setUrl = ubChanger("url");
        const setBody = ubChanger("body");

        let errorElem = <></>;
        if (this.state.error !== undefined) {
            errorElem = (
                <p>
                    An error occurred: {this.state.error}
                </p>
            );
        }

        return (
            <>
                {errorElem}
                <table>
                    <TextField name="title" value={title} setValue={setTitle} />
                    <TextField name="url" value={url} setValue={setUrl} />
                    <TextField name="body" value={body} setValue={setBody} isBody={true} />
                    <TextField name="tags" value={tags} setValue={setTags} />
                    <tr><td><ImageUploader /></td></tr>
                    <tr><td><button onClick={() => this.doUpload()}>Submit</button></td></tr>
                </table>
            </>
        );
    }
};

export default BlogpostEditor;

interface TextFieldProps {
    name: string,
    value: string,
    setValue: (s: string) => void,
    isBody?: boolean,
};

function TextField(props: TextFieldProps) {
    const isBody = props.isBody === true;
    let body;

    if (!isBody) {
        body = (
            <input  type="text" 
                    id={props.name} 
                    value={props.value} 
                    onChange={(ev) => props.setValue((ev!.target! as HTMLInputElement).value)} />
        );
    } else {
        body = (
            <textarea id={props.name}
                      value={props.value}
                      onChange={(ev) => props.setValue((ev!.target! as HTMLInputElement).value)} />
        );
    }

    return (
        <tr>
            <td>
                <label htmlFor={props.name}>{capitalize(props.name)}</label>
            </td>
            <td>
                {body}
            </td>
        </tr>
    );
}