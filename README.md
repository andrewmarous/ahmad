# ahmad
#### an integrated, royalty-free music sample generator

---

ahmad is an experimental interface for a sample generation AI model. While the only current working version is
contained in the `standalone` branch (as a standalone application), the alpha release will be in the form
of a VST3/CLAP plugin.

## building
runpod.io is currently used to host the backend model, which means that every client instance requires two secrets to
function properly: `$API_URL` and `$RUNPOD_API_KEY` (which are not contained in the repository). If you want a copy of ahmad
for yourself, feel free to reach out to me.
