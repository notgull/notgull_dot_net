// GNU AGPL v3 License

import { Component, h } from "preact";

import getConsts from "./consts";
import { Empty } from "./util";
import { postFormData } from "./api";

interface ImageUploaderState {
    category: string,
    subcategory: string,
    filename: string,
    data: any,
    uploading: boolean,
    error: string,
    urls: string[]
}

type ChangeEvent<T> = any;

interface UrlContainer {
    url: string,
}

type IusKey = keyof ImageUploaderState;

export class ImageUploader extends Component<Empty, ImageUploaderState> {
    state = {
        category: "",
        subcategory: "",
        filename: "",
        data: undefined,
        uploading: false,
        error: "",
        urls: [],
    };

    setFile(data: any) {
        this.setState({
            data,
        });
    }

    upload() {
        const { category, subcategory, filename, data } = this.state;
        let error = "";
        if (data === undefined) {
            error = "No file";
        }

        if (error) {
            this.setState({ error });
            return;
        }

        if (!this.state.uploading) {
            this.setState({
                uploading: true,
            });            

            const consts = getConsts();

            const fdata = new FormData();
            fdata.append("category", category);
            fdata.append("subcategory", subcategory);
            fdata.append("filename", filename);
            fdata.append("data", data![0]);
            fdata.append("csrf_token", consts.csrf_token!);
            fdata.append("csrf_cookie", consts.csrf_cookie!);

            postFormData<UrlContainer>("image", fdata).then(u => {
                const urls: string[] = this.state.urls;
                urls.push(`${consts.static_url}/${u.url}`);
                this.setState({
                    urls,
                    uploading: false,
                });
            });
        }
    }

    render() {
        let { category, subcategory, filename, error, urls } = this.state;
        const setCategory = (ev: ChangeEvent<HTMLInputElement>) => this.setState({
            category: ev.target.value,
        });
        const setSubategory = (ev: ChangeEvent<HTMLInputElement>) => this.setState({
            subcategory: ev.target.value,
        });
        const setFilename = (ev: ChangeEvent<HTMLInputElement>) => this.setState({
            filename: ev.target.value,
        });

        const uElems = urls.map(url => (
            <li>{url}</li>
        ));

        return (
            <div className="fileuploader">
                <p>{error}</p>
                <label>Category:</label>
                <input type="text" value={category} onChange={setCategory} />
                <br />
                <label>Subcategory:</label>
                <input type="text" value={subcategory} onChange={setSubategory} />
                <br />
                <label>Filename:</label>
                <input type="text" value={filename} onChange={setFilename} />
                <br />
                <input type="file" onChange={(ev) => this.setFile((ev!.target! as HTMLInputElement).files)} />
                <ul>
                    {uElems}
                </ul>
                <button onClick={() => this.upload()}>Upload</button>
            </div>
        );
    }
};

export default ImageUploader;