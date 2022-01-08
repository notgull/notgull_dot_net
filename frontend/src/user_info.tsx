// GNU AGPL v3 License

import { Component, Fragment, h } from "preact";

import getConsts from "./consts";
import Loading from "./loading";
import { get, patch } from "./api";
import { Empty, LoadingState } from "./util";
import { User } from "./models";

interface UserInfoState {
    loadstate: LoadingState,
    user: User | undefined,
    error: string,
    uploading: boolean,
}

export class UserInfo extends Component<Empty, UserInfoState> {
    state = {
        loadstate: LoadingState.Unmounted,
        user: undefined,
        error: "",
        uploading: false,
    };

    componentDidMount() {
        this.setState({
            loadstate: LoadingState.Loading,
        });

        const { cur_user_id } = getConsts();
        get<User>("user", cur_user_id!).then(user => {
            this.setState({
                loadstate: LoadingState.Loaded,
                user,
            });
        });
    }

    patchUser() {
        if (!this.state.uploading) {
            this.setState({
                uploading: true,
            });

            const { cur_user_id } = getConsts();
            patch<User>("user", cur_user_id!, this.state.user!).then(() => {
                window.location.href = "/admin/users";
            });
        }
    }

    render() {
        const { loadstate, user, error } = this.state;
        if (loadstate == LoadingState.Unmounted) {
            return <></>;
        } else if (loadstate == LoadingState.Loading) {
            return <Loading />;
        } else if (loadstate == LoadingState.Loaded) {
            const u: User = user!;
            const setName = (name: string) => this.setState({
                user: Object.assign(u, { name }),
            });
            const setUuid = (uuid: string) => this.setState({
                user: Object.assign(u, { uuid }),
            });
            const updateRoles = (index: number, isGiven: boolean) => {
                let roles = u!.roles;
                if (isGiven) {
                    roles |= 1 << index;
                } else {
                    roles &= ~(1 << index);
                }
                this.setState({
                    user: Object.assign(u, { roles }),
                });
            };

            return (
                <UserForm
                    user={u}
                    setName={setName}
                    setUuid={setUuid}
                    updateRoles={updateRoles} 
                    doSubmit={() => this.patchUser()} />
            );
        } else {
            return (
                <p>Error: {this.state.error}</p>
            );
        }
    }
};

export default UserInfo;

interface UserFormProps {
    user: User,
    setName: (s: string) => void,
    setUuid: (s: string) => void,
    updateRoles: (index: number, isGiven: boolean) => void,
    doSubmit: () => void,
};

function UserForm(props: UserFormProps) {
    const { user, setName, setUuid, updateRoles, doSubmit } = props;

    const roles = ["Blogger", "Admin"];
    const permCheckboxes = roles.map((role, i) => {
        const isChecked = (user.roles & (1 << i)) != 0; 
        const setChecked = (c: boolean) => updateRoles(i, c);

        return (
            <>
                <PermCheckbox isChecked={isChecked} setChecked={setChecked} name={role} />
                <br />
            </>
        );
    });

    return (
        <>
            <label>Name:</label>
            <input type="text" value={user.name} onChange={(ev) => setName((ev!.target! as HTMLInputElement).value)} />
            <br />
            <label>UUID:</label>
            <input type="text" value={user.uuid} onChange={(ev) => setUuid((ev!.target! as HTMLInputElement).value)} />
            <br />
            <label>Roles:</label>
            <br />
            {permCheckboxes}
            <button onClick={doSubmit}>Submit</button>
        </>
    );
}

interface PermCheckboxProps {
    isChecked: boolean,
    setChecked: (checked: boolean) => void,
    name: string,
};

function PermCheckbox(props: PermCheckboxProps) {
    const { isChecked, setChecked, name } = props;

    return (
        <>
            <input type="checkbox" checked={isChecked} onChange={(ev) => setChecked((ev!.target! as HTMLInputElement).checked)} />
            <label>{name}</label>
        </>
    );
}