import { LockIcon, UserIcon, WarningIcon } from "@phosphor-icons/react";
import SectionedModal, {
	ModalSection,
} from "../../../components/AppModal/SectionedModal";
import { useAppModal } from "../../../hooks/useAppModal";
import ProfileTab from "./ProfileTab";
import PasswordTab from "./PasswordTab";
import DangerZoneTab from "./DangerZoneTab";

const SECTIONS: ModalSection[] = [
	{
		value: "profile",
		label: "Profile",
		icon: <UserIcon />,
		render: () => <ProfileTab />,
	},
	{
		value: "password",
		label: "Password",
		icon: <LockIcon />,
		render: () => <PasswordTab />,
	},
	{
		value: "danger-zone",
		label: "Danger zone",
		icon: <WarningIcon />,
		render: () => <DangerZoneTab />,
	},
];

function ManageAccountModal() {
	const { opened, close } = useAppModal("manageAccount");

	return (
		<SectionedModal
			opened={opened}
			onClose={close}
			title="Manage account"
			navAriaLabel="Open account navigation"
			sections={SECTIONS}
			size="xl"
		/>
	);
}

export default ManageAccountModal;
