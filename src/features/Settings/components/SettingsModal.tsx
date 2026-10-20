import { DatabaseIcon, InfoIcon, PaletteIcon } from "@phosphor-icons/react";
import { AiIcon } from "../../../config/icons";
import SectionedModal, {
	ModalSection,
} from "../../../components/AppModal/SectionedModal";
import { useAppModal } from "../../../hooks/useAppModal";
import AppearanceTab from "./AppearanceTab";
import DataTab from "./DataTab";
import AboutTab from "./AboutTab";
import AiTab from "./AiTab";

const SECTIONS: ModalSection[] = [
	{
		value: "appearance",
		label: "Appearance",
		icon: <PaletteIcon />,
		render: () => <AppearanceTab />,
	},
	{
		value: "data",
		label: "Data",
		icon: <DatabaseIcon />,
		render: () => <DataTab />,
	},
	{
		value: "ai",
		label: "AI",
		icon: <AiIcon />,
		render: () => <AiTab />,
	},
	{
		value: "about",
		label: "About",
		icon: <InfoIcon />,
		render: () => <AboutTab />,
	},
];

function SettingsModal() {
	const { opened, close } = useAppModal("settings");

	return (
		<SectionedModal
			opened={opened}
			onClose={close}
			title="Settings"
			navAriaLabel="Open settings navigation"
			sections={SECTIONS}
			size="xl"
		/>
	);
}

export default SettingsModal;
