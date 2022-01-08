// GNU AGPL v3 License

import { Component, Fragment, h } from "preact";

import getConsts from "./consts";
import { Empty } from "./util";
import { post } from "./api";

interface UsernameFormState {
    username: string,
    sending: boolean,
};

export class UsernameForm extends Component<Empty, UsernameFormState> {
    state = {
        username: "",
        sending: false,
    };

    sendUsername() {
        this.setState({ sending: true });
        post("username", { username: this.state.username }).then(() => {
            window.location.href = getConsts().web_url;
        });
    }

    render() {
        const value = this.state.username;
        const setValue = (v: string) => { this.setState({ username: v }); };

        return (
            <>
                <p>
                    You have not yet entered a username. Please enter your username.
                </p>
                <input type="text" value={value} onChange={(ev) => setValue((ev!.target! as HTMLInputElement).value)} />
                <button onClick={() => this.sendUsername()}>Submit</button>
            </>
        )
    }
};

export default UsernameForm;