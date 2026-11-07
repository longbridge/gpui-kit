<script lang="ts">
	// Keep the following line to suppress type checking errors in this example
	// @ts-nocheck
	import { fade, slide } from 'svelte/transition';

	let state = $state({
		count: 0,
		name: 'World',
		items: ['Apple', 'Banana', 'Cherry'],
		newItem: ''
	});

	// Those lines raise a warning in the IDE, uncomment in the editor example to see them highlighted
	// let summary = $derived(
	// 	`Hello ${state.name}! Count: ${state.count}, Items: ${state.items.length}`
	// );
	// $effect(() => {
	// 	console.log(`State updated:`, state);
	// });

	function increment() {
		state.count++;
	}

	function decrement() {
		state.count--;
	}

	function reset() {
		state.count = 0;
		state.name = 'World';
	}

	function addItem() {
		if (state.newItem.trim()) {
			state.items.push(state.newItem.trim());
			state.newItem = '';
		}
	}

	function removeItem(index: number) {
		state.items.splice(index, 1);
	}

	async function asyncIncrement() {
		await new Promise((resolve) => setTimeout(resolve, 500));
		state.count++;
	}
</script>

<main class="p-6 max-w-2xl mx-auto">
	<h1 class="text-3xl font-bold mb-6">Svelte 5 Features</h1>

	<section class="mb-6 p-4 border rounded">
		<h2 class="text-xl font-semibold mb-3">Counter</h2>
		<p class="mb-4">Count: <strong>{state.count}</strong></p>

		<div class="flex gap-2">
			<button onclick={increment} class="px-4 py-2 bg-blue-500 text-white rounded">
				Increment
			</button>
			<button onclick={decrement} class="px-4 py-2 bg-red-500 text-white rounded">
				Decrement
			</button>
			<button onclick={asyncIncrement} class="px-4 py-2 bg-green-500 text-white rounded">
				Async +1
			</button>
			<button onclick={reset} class="px-4 py-2 bg-gray-500 text-white rounded">Reset</button>
		</div>
	</section>

	<section class="mb-6 p-4 border rounded">
		<h2 class="text-xl font-semibold mb-3">Input Binding</h2>
		<input
			type="text"
			bind:value={state.name}
			placeholder="Enter your name"
			class="px-3 py-2 border rounded w-full"
		/>
	</section>

	<section class="mb-6 p-4 border rounded">
		<h2 class="text-xl font-semibold mb-3">List Management</h2>

		<div class="flex gap-2 mb-4">
			<input
				type="text"
				bind:value={state.newItem}
				placeholder="Add item"
				class="px-3 py-2 border rounded grow"
				onkeydown={(e) => e.key === 'Enter' && addItem()}
			/>
			<button onclick={addItem} class="px-4 py-2 bg-blue-500 text-white rounded">Add</button>
		</div>

		{#if state.items.length === 0}
			<p class="text-gray-500" transition:fade>No items yet. Add some!</p>
		{:else}
			<ul class="space-y-2">
				{#each state.items as item, i (item)}
					<li
						class="flex justify-between items-center p-2 bg-gray-100 rounded"
						transition:slide
					>
						<span>{item}</span>
						<button
							onclick={() => removeItem(i)}
							class="px-2 py-1 bg-red-500 text-white rounded text-sm"
						>
							Remove
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</section>
</main>
