import { Button, Text } from "@mantine/core";
import useAppDispatch from "../../../hooks/useAppDispatch";
import useAppSelector from "../../../hooks/useAppSelector";
import { openModal } from "../../../stores/app/appReducer";
import {
	selectIsSignedIn,
	selectUserInformation,
} from "../../../stores/user/userSelectors";

function VerifyEmailBanner() {
	const dispatch = useAppDispatch();
	const isSignedIn = useAppSelector(selectIsSignedIn);
	const isEmailVerified = useAppSelector(
		state => selectUserInformation(state)?.isEmailVerified,
	);

	if (!isSignedIn || isEmailVerified) return null;

	return (
		<Button onClick={() => dispatch(openModal("verifyEmail"))} mx="sm">
			<Text>Verify your email</Text>
		</Button>
	);
}

export default VerifyEmailBanner;
