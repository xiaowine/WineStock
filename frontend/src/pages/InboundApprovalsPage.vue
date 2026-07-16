<!-- 本路由页只选择入库审批领域并挂载共享工作台；不复制队列、详情或审批请求。 -->
<template><StockApprovalWorkspace kind="inbound" /></template>
<script setup lang="ts">
import StockApprovalWorkspace from '../components/approvals/StockApprovalWorkspace.vue'
</script>
