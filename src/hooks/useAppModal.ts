import { closeModal, ModalName } from "../stores/app/appReducer";
import useAppDispatch from "./useAppDispatch";
import useAppSelector from "./useAppSelector";

export function useAppModal(name: ModalName) {
	const dispatch = useAppDispatch();
	const opened = useAppSelector(state => state.app.openModals[name] === true);
	return { opened, close: () => dispatch(closeModal(name)) };
}
