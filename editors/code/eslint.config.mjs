import eslint from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
	{
		ignores: ["out/**", "dist/**", "**/*.d.ts"],
	},
	eslint.configs.recommended,
	...tseslint.configs.recommended,
	{
		files: ["src/**/*.ts"],
		rules: {
			"@typescript-eslint/naming-convention": [
				"warn",
				{
					selector: "import",
					format: ["camelCase", "PascalCase"],
				},
			],
			"@typescript-eslint/no-unused-vars": [
				"warn",
				{
					argsIgnorePattern: "^_",
					varsIgnorePattern: "^_",
				},
			],
			"curly": "warn",
			"eqeqeq": "warn",
			"no-throw-literal": "warn",
		},
	},
);
