# PerchFlow Docker V1 — 开发与验收

当前状态：feature/docker-v1 开发分支，尚未合并 main，也未部署。

## 功能

本机 Docker 环境检测、全部容器列表（包括停止容器）、状态、CPU 与内存、详情、日志及启动/停止/重启。页面沿用现有 PerchFlow 顶部与移动端导航，Docker 页面只分容器列表和容器详情两层。

Docker 环境检测通过本机 CLI 完成。服务账户需有访问 Windows Docker Engine 命名管道的权限。若 CLI 不存在或 Engine 不可达，返回错误，不将缓存结果显示成实时状态。路径按 PERCHFLOW_DOCKER_CLI、DOCKER_CLI_PATH、常见安装位置、PATH 查找；覆盖路径必须是绝对路径且由服务账户设置，浏览器端不能指定。只允许本机 named pipe 或 loopback Docker endpoint，拒绝远程 Docker context / DOCKER_HOST。

## API

- GET /api/docker/environment — Docker CLI 和 Engine 检测
- GET /api/docker/containers — docker ps -a、docker inspect、docker stats --no-stream
- GET /api/docker/containers/:id — docker inspect
- GET /api/docker/containers/:id/logs?tail=200 — docker logs 最近 1–1000 行，合并 stdout/stderr
- POST /api/docker/containers/:id/start — docker start
- POST /api/docker/containers/:id/stop — docker stop --time 10
- POST /api/docker/containers/:id/restart — docker restart --time 10

所有 API 由既有 Bearer Token middleware 保护。操作始终使用完整 64 位十六进制容器 ID，不通过 shell 执行任何用户参数；操作后使用 Docker inspect 验证真实状态。写入操作使用互斥锁串行化并设置超时。停止和重启需要界面二次确认。

## 开销控制

状态和资源每 5 秒刷新，页面不可见/上一请求仍在执行时不发起新请求。仅对运行中容器调用 Docker stats；已停止容器资源显示为 —。失败不会冒充已停止。日志只在详情页拉取，默认最近 200 行，最大 1000 行，具有自动刷新开关。V1 不管理镜像、卷、网络、Compose，不安装或修改 Docker Engine，不管理远程 Docker，也不接管容器重启策略。

## 验证

CI 的 fake_docker_v1.rs 模拟 Docker CLI，verify_docker_v1.py 以完全独立的 PerchFlow 进程与临时配置验证认证、状态、运行中与停止容器、资源、日志、启停重启、非法 ID 和 Engine 断线。CI 不使用真实 Docker 容器。

真实环境验收顺序：先检查运行账户 Docker CLI 可用和权限，再只读查看全部容器，最后在专用测试容器执行启停重启。Ydisks 等正在使用的业务容器只允许只读状态与日志检查，未经单独确认不得停止或重启。
