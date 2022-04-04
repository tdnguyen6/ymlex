.ONESHELL:
.SILENT:

overlay-annotate:
	if ! grep -qxFf configs/overlay-annotations.txt configs/overlay.ymlex.yml; then
		tmp_file=$$(mktemp)
		cat configs/overlay-annotations.txt configs/overlay.ymlex.yml > $$tmp_file
		mv $$tmp_file configs/overlay.ymlex.yml
	fi

generate-config: overlay-annotate
	ytt -f configs/overlay.ymlex.yml -f configs/default.ymlex.yml > configs/main.ymlex.yml
