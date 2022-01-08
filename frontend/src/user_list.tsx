// GNU AGPL v3 License

import { Component, Fragment, h } from "preact";

import Loading from "./loading";
import Pagination from "./pagination";
import { Empty, LoadingState } from "./util";
import { list } from "./api";
import { User } from "./models";

interface UserListState {
    users: User[],
    loadstate: LoadingState,
    name: string,
    page_size: number,
    page_index: number,
};

export class UserList extends Component<Empty, UserListState> {
    state = {
        users: [],
        loadstate: LoadingState.Unmounted,
        name: "",
        page_size: 25,
        page_index: 0,
    };

    private loadUsers() {
        this.setState({
            loadstate: LoadingState.Loading,
        });

        list<User>("user", { 
            name: this.state.name,
            skip: this.state.page_size * this.state.page_index,
            count: this.state.page_size,
        }).then((users) => {
            this.setState({
                users,
                loadstate: LoadingState.Loaded,
            });
        });
    }

    componentDidMount() {
        this.loadUsers();
    }

    render() {
        if (this.state.loadstate == LoadingState.Loading) {
            return <Loading />;
        } else if (this.state.loadstate == LoadingState.Unmounted) {
            return <></>;
        } 

        const userEntries = this.state.users.map((user) => (
            UserListItem({ user })
        ));

        const setPage = (page: number) => {
            this.setState({
                page_index: page,
            });

            this.loadUsers();
        };

        return (
            <>
                <table>
                    <tbody>
                        <tr>
                            <th>ID</th>
                            <th>Name</th>
                            <th>UUID</th>
                            <th>Roles</th>
                        </tr>
                        {userEntries}
                    </tbody>
                </table>
                <Pagination
                  page={this.state.page_index}
                  setPage={setPage} />
            </>
        );
    }
};

export default UserList;

interface UserListItemProps {
    user: User,
};

function UserListItem(props: UserListItemProps) {
    return (
        <tr key={props.user.id}>
            <td>
                {props.user.id}
            </td>
            <td>
                <a href={`/admin/users/${props.user.id}`}>{props.user.name}</a>
            </td>
            <td>
                {props.user.uuid}
            </td>
            <td>
                0x{props.user.roles.toString(16)}
            </td>
        </tr>
    );
}