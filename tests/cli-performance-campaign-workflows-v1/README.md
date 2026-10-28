These four YAML files are inert test data for the prospective three-phase performance campaign. They are outside `.github/workflows` and are not dispatchable. Their exact byte counts and SHA256 digests are fixed by `cli-performance-campaign-plan-v1.mjs`; API fixtures use those bytes to test source and workflow identity.

The numeric anchor runner and final campaign admission are not implemented here. Copying these files into active workflows requires completing and reviewing those dependencies first. Passing synthetic tests does not qualify hosted measurements or close #418.
